// SPDX-License-Identifier: Apache-2.0
//! RV Studio Render native Rust UI with three distinct, functional view modes.
//! Solid viewport raycasts actual BVH geometry. Live preview uses low-sample
//! CPU path tracing on scene edits and progressively refines. Final view keeps
//! the full-resolution image, with a permanent route back to the environment.
//! None of these paths pretends to be a GPU-backed Cycles viewport.
mod theme;
use crate::preview;

use std::{path::PathBuf,sync::{Arc,mpsc,atomic::{AtomicU64,Ordering}},time::{Duration,Instant}};
use eframe::egui;
use crate::integrator::{
    CameraSettings,RenderSettings,render_rgb_denoised_with_camera,
    render_rgb_denoised_with_camera_cancellable
};
use crate::scene::loader::{load_scene,SceneLoadOptions,LoadedScene};
use crate::film::save_image;
use crate::util::math::Vec3;
use crate::navigation::{orbit,pan,zoom};
use theme::Tokens;

#[derive(Clone,Copy,PartialEq,Eq,Debug)]
enum ViewMode { Solid, Live, Final }
#[derive(Clone,Copy,PartialEq,Eq)]
enum NavigationTool { Orbit, Pan }
#[derive(Clone,Copy,Debug)]
enum Job { Solid, Live(u8), Final }
struct Report{
    job:Job,
    revision:u64,
    output:Option<String>,
    result:Result<(Vec<u8>,u32,u32),String>,
}
struct StudioApp {
    colors:Tokens,
    options:SceneLoadOptions,
    obj_path:String,mtl_path:String,environment_path:String,output_path:String,
    settings:RenderSettings,camera:CameraSettings,denoise:u8,active_material:usize,
    scene:Option<Arc<LoadedScene>>,
    view:ViewMode,revision:u64,
    nav_tool:NavigationTool,
    epoch:Arc<AtomicU64>,last_edit:Instant,
    solid:Option<egui::TextureHandle>,
    live:Option<egui::TextureHandle>,
    final_image:Option<egui::TextureHandle>,
    solid_revision:Option<u64>,
    live_revision:Option<u64>,
    live_pass:u8,
    solid_busy:bool,live_busy:bool,final_busy:bool,
    sender:mpsc::Sender<Report>,receiver:mpsc::Receiver<Report>,
    status:String,
}
impl StudioApp {
    fn new(cc:&eframe::CreationContext<'_>)->Self{
        let colors=Tokens::palette(true);
        colors.apply(&cc.egui_ctx);
        let (sender,receiver)=mpsc::channel();
        Self{
            colors,options:SceneLoadOptions::studio(),
            obj_path:String::new(),mtl_path:String::new(),environment_path:String::new(),
            output_path:"RV_Studio_Render.png".into(),
            settings:RenderSettings{
                width:960,height:540,samples:24,max_bounces:5,exposure:1.2,seed:17
            },
            camera:CameraSettings::default(),denoise:2,active_material:0,
            scene:load_scene(&SceneLoadOptions::studio()).ok().map(Arc::new),
            view:ViewMode::Solid,revision:1,
            nav_tool:NavigationTool::Orbit,
            epoch:Arc::new(AtomicU64::new(1)),last_edit:Instant::now(),
            solid:None,live:None,final_image:None,
            solid_revision:None,live_revision:None,live_pass:0,
            solid_busy:false,live_busy:false,final_busy:false,
            sender,receiver,
            status:"Pré-visualização sólida disponível sem renderizar.".into(),
        }
    }
    fn invalidate(&mut self){
        self.revision=self.revision.saturating_add(1);
        self.epoch.store(self.revision,Ordering::Relaxed);
        self.last_edit=Instant::now();
        self.solid_revision=None;
        self.live_revision=None;
        self.live_pass=0;
    }
    fn load(&mut self){
        let obj=self.obj_path.trim();
        let mtl=self.mtl_path.trim();
        let environment=self.environment_path.trim();
        self.options.obj=(!obj.is_empty()).then(||PathBuf::from(obj));
        self.options.mtl=(!mtl.is_empty()).then(||PathBuf::from(mtl));
        self.options.environment=(!environment.is_empty()).then(||PathBuf::from(environment));
        match load_scene(&self.options){
            Ok(loaded)=>{
                self.status=format!("Cena: {} • {} triângulos • {} materiais",
                    loaded.source_label,loaded.scene.bvh.stats().triangles,
                    loaded.material_names.len());
                self.scene=Some(Arc::new(loaded));
                self.active_material=0;
                self.final_image=None;
                self.view=ViewMode::Solid;
                self.invalidate();
            }
            Err(e)=>self.status=format!("Não foi possível importar a cena: {e}"),
        }
    }
    fn launch_final(&mut self){
        if self.final_busy {
            self.view=ViewMode::Final;
            return;
        }
        let Some(snapshot)=self.scene.as_ref().cloned() else{
            self.status="Abra uma cena antes de renderizar.".into();return;
        };
        if let Err(e)=self.settings.validate(){self.status=e.into();return;}
        if let Err(e)=self.camera.validate(){self.status=e.into();return;}
        let settings=self.settings;
        let camera=self.camera;
        let denoise=self.denoise;
        let output=self.output_path.clone();
        let revision=self.revision;
        let tx=self.sender.clone();
        self.final_busy=true;
        self.view=ViewMode::Final;
        self.status=format!("Render final CPU {}×{} • {} amostras. Pode voltar ao ambiente.",
            settings.width,settings.height,settings.samples);
        std::thread::spawn(move||{
            let result=(||->Result<(Vec<u8>,u32,u32),String>{
                let bytes=render_rgb_denoised_with_camera(
                    &snapshot.scene,settings,camera,denoise
                ).map_err(str::to_owned)?;
                save_image(&output,&bytes,settings.width,settings.height)
                    .map_err(|e|format!("Não foi possível salvar o arquivo: {e}"))?;
                Ok((bytes,settings.width,settings.height))
            })();
            let _=tx.send(Report{job:Job::Final,revision,output:Some(output),result});
        });
    }
    fn start_solid(&mut self){
        if self.solid_busy || self.solid_revision==Some(self.revision){return;}
        let Some(snapshot)=self.scene.as_ref().cloned() else{return;};
        let camera=self.camera;
        let (w,h)=preview_size(self.settings,580);
        let tx=self.sender.clone();
        let revision=self.revision;
        let epoch=self.epoch.clone();
        self.solid_busy=true;
        std::thread::spawn(move||{
            let result=preview::solid_preview_cancellable(
                &snapshot.scene,camera,w,h,&epoch,revision
            ).map(|bytes|(bytes,w,h)).map_err(str::to_owned);
            let _=tx.send(Report{job:Job::Solid,revision,output:None,result});
        });
    }
    fn start_live(&mut self){
        if self.live_busy || self.live_pass>=3{return;}
        let Some(snapshot)=self.scene.as_ref().cloned() else{return;};
        let mut settings=self.settings;
        let (w,h)=preview_size(settings,360);
        settings.width=w;
        settings.height=h;
        settings.samples=[1,2,4][self.live_pass as usize];
        settings.max_bounces=settings.max_bounces.min(4);
        let pass=self.live_pass;
        let revision=self.revision;
        let camera=self.camera;
        let tx=self.sender.clone();
        let epoch=self.epoch.clone();
        self.live_busy=true;
        std::thread::spawn(move||{
            let result=render_rgb_denoised_with_camera_cancellable(
                &snapshot.scene,settings,camera,if pass==2 {1}else{0},
                &epoch,revision
            ).map(|bytes|(bytes,w,h)).map_err(str::to_owned);
            let _=tx.send(Report{job:Job::Live(pass),revision,output:None,result});
        });
    }
    fn receive(&mut self,ctx:&egui::Context){
        while let Ok(report)=self.receiver.try_recv(){
            match report.job{
                Job::Solid=>self.solid_busy=false,
                Job::Live(_)=>self.live_busy=false,
                Job::Final=>self.final_busy=false,
            }
            if !matches!(report.job,Job::Final) && report.revision!=self.revision{
                continue; // Discard stale previews from before a camera/edit event.
            }
            match report.result{
                Ok((rgb,w,h))=>{
                    let image=egui::ColorImage::from_rgb([w as usize,h as usize],&rgb);
                    let tag=match report.job {
                        Job::Solid=>"RV Solid",
                        Job::Live(_)=>"RV Live",
                        Job::Final=>"RV Final"
                    };
                    let texture=ctx.load_texture(tag,image,egui::TextureOptions::LINEAR);
                    match report.job{
                        Job::Solid=>{
                            self.solid=Some(texture);
                            self.solid_revision=Some(report.revision);
                        }
                        Job::Live(pass)=>{
                            self.live=Some(texture);
                            self.live_revision=Some(report.revision);
                            self.live_pass=pass+1;
                            if self.view==ViewMode::Live{
                                self.status=format!("Render em tempo real CPU: {} amostras ({}×{}).",
                                    [1,2,4][pass as usize],w,h);
                            }
                        }
                        Job::Final=>{
                            self.final_image=Some(texture);
                            self.status=format!("Render concluído: {} ({}×{}).",
                                report.output.unwrap_or_default(),w,h);
                        }
                    }
                }
                Err(e)=>self.status=format!("Erro de renderização: {e}"),
            }
        }
    }
    fn top(&mut self,ctx:&egui::Context){
        egui::TopBottomPanel::top("rv_topbar")
            .show(ctx,|ui|{
                ui.horizontal_wrapped(|ui|{
                    ui.heading(egui::RichText::new("RV STUDIO RENDER")
                        .size(self.colors.brand_size));
                    ui.label(egui::RichText::new("CirclesRust • CPU")
                        .color(self.colors.muted));
                    ui.separator();
                    if ui.selectable_label(self.view==ViewMode::Solid,"Ambiente 3D").clicked(){
                        self.view=ViewMode::Solid;
                    }
                    if ui.selectable_label(self.view==ViewMode::Live,"Tempo real").clicked(){
                        self.view=ViewMode::Live;
                    }
                    if ui.add_enabled(
                        self.final_image.is_some()||self.final_busy,
                        egui::SelectableLabel::new(self.view==ViewMode::Final,"Imagem final")
                    ).clicked(){self.view=ViewMode::Final;}
                    if ui.add_enabled(!self.final_busy,
                        egui::Button::new(egui::RichText::new("RENDERIZAR")
                            .strong().color(self.colors.accent))
                    ).clicked(){self.launch_final();}
                    if self.view==ViewMode::Final &&
                        ui.button("Voltar ao ambiente").clicked(){
                        self.view=ViewMode::Solid;
                    }
                    if ui.button(if self.colors.dark{"Tema claro"}else{"Tema escuro"}).clicked(){
                        self.colors=Tokens::palette(!self.colors.dark);
                        self.colors.apply(ctx);
                    }
                });
                ui.horizontal_wrapped(|ui|{
                    ui.label("Resolução");
                    ui.add(egui::DragValue::new(&mut self.settings.width)
                        .range(1..=4096).suffix(" px"));
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut self.settings.height)
                        .range(1..=4096).suffix(" px"));
                    ui.separator();
                    ui.label("Amostras");
                    ui.add(egui::DragValue::new(&mut self.settings.samples)
                        .range(1..=1024));
                    ui.label("Ressaltos");
                    ui.add(egui::DragValue::new(&mut self.settings.max_bounces)
                        .range(1..=32));
                    ui.label(if self.final_busy{"Render final em execução..."}else{
                        "Prévia e render final independentes"
                    });
                });
            });
    }
    fn left(&mut self,ctx:&egui::Context){
        let max_side=(ctx.available_rect().width()*0.38)
            .clamp(self.colors.min_side,self.colors.max_side);
        egui::SidePanel::left("rv_environment")
            .resizable(true)
            .width_range(self.colors.min_side..=max_side)
            .default_width(self.colors.left_width.min(max_side))
            .show(ctx,|ui|{
                egui::ScrollArea::vertical().show(ui,|ui|{
                    ui.add_space(self.colors.padding);
                    ui.heading("Ambiente / Iluminação");
                    ui.label("Mapa HDR / PNG / JPEG");
                    ui.add(egui::TextEdit::singleline(&mut self.environment_path)
                        .desired_width(f32::INFINITY));
                    ui.add(egui::Slider::new(
                        &mut self.options.environment_strength,0.0..=15.0)
                        .text("Intensidade"));
                    if ui.button("Aplicar ambiente").clicked(){self.load();}
                    let mut changed=false;
                    if let Some(scene)=self.scene.as_mut(){
                        ui.separator();
                        ui.label("Luz de área");
                        let scene=&mut Arc::make_mut(scene).scene;
                        changed|=ui.add(egui::Slider::new(
                            &mut scene.light.center.z,1.0..=12.0).text("Altura")).changed();
                        changed|=ui.add(egui::Slider::new(
                            &mut scene.light.radiance.x,0.0..=20.0).text("Luz R")).changed();
                        changed|=ui.add(egui::Slider::new(
                            &mut scene.light.radiance.y,0.0..=20.0).text("Luz G")).changed();
                        changed|=ui.add(egui::Slider::new(
                            &mut scene.light.radiance.z,0.0..=20.0).text("Luz B")).changed();
                    }
                    if changed{self.invalidate();}
                    ui.separator();
                    ui.heading("Objetos");
                    ui.label("Modelo OBJ");
                    ui.add(egui::TextEdit::singleline(&mut self.obj_path)
                        .desired_width(f32::INFINITY));
                    ui.label("Material MTL (opcional)");
                    ui.add(egui::TextEdit::singleline(&mut self.mtl_path)
                        .desired_width(f32::INFINITY));
                    if ui.button("Importar / Recarregar").clicked(){self.load();}
                    if let Some(scene)=&self.scene{
                        ui.separator();
                        ui.label(&scene.source_label);
                        ui.label(format!("{} triângulos",scene.scene.bvh.stats().triangles));
                        for (id,name) in scene.material_names.iter().enumerate(){
                            ui.selectable_value(&mut self.active_material,id,name);
                        }
                    }
                });
            });
    }
    fn right(&mut self,ctx:&egui::Context){
        let max_side=(ctx.available_rect().width()-self.colors.min_center)
            .clamp(self.colors.min_side,self.colors.max_side);
        egui::SidePanel::right("rv_materials")
            .resizable(true)
            .width_range(self.colors.min_side..=max_side)
            .default_width(self.colors.right_width.min(max_side))
            .show(ctx,|ui|{
                egui::ScrollArea::vertical().show(ui,|ui|{
                    ui.add_space(self.colors.padding);
                    ui.heading("Propriedades / Materiais");
                    let Some(loaded)=&self.scene else {
                        ui.label("Abra uma cena.");return;
                    };
                    let id=self.active_material.min(loaded.scene.materials.len()-1);
                    ui.label(&loaded.material_names[id]);
                    let mut changed=false;
                    if let Some(scene)=self.scene.as_mut(){
                        let material=&mut Arc::make_mut(scene).scene.materials[id];
                        ui.label("Cor base");
                        let mut values=[
                            material.base_color.x,material.base_color.y,material.base_color.z
                        ];
                        if ui.color_edit_button_rgb(&mut values).changed(){
                            material.base_color=Vec3::new(values[0],values[1],values[2]);
                            changed=true;
                        }
                        changed|=ui.add(egui::Slider::new(
                            &mut material.roughness,0.045..=1.0).text("Rugosidade")).changed();
                        changed|=ui.add(egui::Slider::new(
                            &mut material.metallic,0.0..=1.0).text("Metal")).changed();
                    }
                    if changed{self.invalidate();}
                    ui.separator();
                    ui.label("Texturas PBR importadas do MTL.");
                    ui.label("Alterações são mostradas no modo Ambiente e Tempo real.");
                });
            });
    }
    fn camera_panel(&mut self,ctx:&egui::Context){
        egui::TopBottomPanel::bottom("rv_camera")
            .resizable(true)
            .height_range(self.colors.camera_min..=self.colors.camera_max)
            .default_height(self.colors.camera_height)
            .show(ctx,|ui|{
                ui.heading("Câmera");
                ui.horizontal_wrapped(|ui|{
                    ui.vertical(|ui|{
                        ui.label("Posição");
                        if vector(ui,&mut self.camera.origin){self.invalidate();}
                    });
                    ui.vertical(|ui|{
                        ui.label("Alvo");
                        if vector(ui,&mut self.camera.target){self.invalidate();}
                    });
                    ui.vertical(|ui|{
                        ui.label("Campo de visão");
                        if ui.add(egui::Slider::new(
                            &mut self.camera.fov_degrees,5.0..=150.0)
                            .text("FOV °")).changed(){self.invalidate();}
                        ui.add(egui::Slider::new(
                            &mut self.settings.exposure,0.1..=4.0)
                            .text("Exposição"));
                    });
                });
            });
    }
    fn center(&mut self,ctx:&egui::Context){
        egui::CentralPanel::default().show(ctx,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.heading(match self.view {
                    ViewMode::Solid=>"Ambiente • Objetos sólidos",
                    ViewMode::Live=>"Tempo real • Path tracing CPU",
                    ViewMode::Final=>"Render final • Imagem",
                });
                if self.view==ViewMode::Final&&ui.button("Voltar para pré-visualização").clicked(){
                    self.view=ViewMode::Solid;
                }
            });
            ui.label(egui::RichText::new(match self.view{
                ViewMode::Solid=>"Botão do meio: orbitar • Shift + meio: pan • Roda: zoom",
                ViewMode::Live=>"Tempo real CPU: 1 → 2 → 4 amostras • Shift + meio: pan",
                ViewMode::Final=>"Imagem exportada em resolução original • Renderização CPU",
            }).color(self.colors.muted));
            ui.separator();
            if self.view!=ViewMode::Final {
                ui.horizontal_wrapped(|ui|{
                    ui.label("Navegação");
                    ui.selectable_value(&mut self.nav_tool,NavigationTool::Orbit,"Orbitar");
                    ui.selectable_value(&mut self.nav_tool,NavigationTool::Pan,"Pan");
                    if ui.button("Redefinir câmera").clicked(){
                        self.camera=CameraSettings::default();
                        self.invalidate();
                    }
                    ui.label("Arraste com o botão esquerdo usando a ferramenta selecionada");
                });
            }
            if self.view==ViewMode::Final {
                ui.label("Arquivo final");
                ui.add(egui::TextEdit::singleline(&mut self.output_path)
                    .desired_width(f32::INFINITY));
                ui.add(egui::Slider::new(&mut self.denoise,0..=5).text("Denoise"));
            }
            let space=ui.available_size();
            let (rect,response)=ui.allocate_exact_size(space,egui::Sense::drag());
            let painter=ui.painter_at(rect);
            painter.rect_filled(rect,0.0,self.colors.canvas);
            let texture=match self.view {
                ViewMode::Solid=>self.solid.as_ref(),
                ViewMode::Live=>self.live.as_ref(),
                ViewMode::Final=>self.final_image.as_ref(),
            };
            if let Some(texture)=texture{
                let original=texture.size_vec2();
                let scale=(rect.width()/original.x).min(rect.height()/original.y).max(0.01);
                let width=original.x*scale;
                let height=original.y*scale;
                let top_left=egui::pos2(rect.center().x-width*0.5,rect.center().y-height*0.5);
                let image_rect=egui::Rect::from_min_size(top_left,egui::vec2(width,height));
                painter.image(texture.id(),image_rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),
                    egui::Color32::WHITE);
            }else{
                let note=match self.view {
                    ViewMode::Solid=>"Preparando geometria do ambiente...",
                    ViewMode::Live=>"Preparando primeira amostra do render em tempo real...",
                    ViewMode::Final=>"Renderizando imagem final... Você pode voltar ao ambiente.",
                };
                painter.text(rect.center(),egui::Align2::CENTER_CENTER,note,
                    egui::FontId::proportional(self.colors.brand_size),self.colors.muted);
            }
            if self.view!=ViewMode::Final{
                let mut changed=false;
                let using_middle=response.dragged_by(egui::PointerButton::Middle);
                let using_primary=response.dragged_by(egui::PointerButton::Primary);
                let shift=ui.input(|i|i.modifiers.shift);
                if using_middle||using_primary{
                    let delta=ui.input(|i|i.pointer.delta());
                    if delta.length_sq()>0.0{
                        let do_pan=if using_middle {shift}
                            else{self.nav_tool==NavigationTool::Pan};
                        changed|=if do_pan{
                            pan(&mut self.camera,delta.x,delta.y,rect.height())
                        }else{
                            orbit(&mut self.camera,delta.x,delta.y)
                        };
                    }
                }
                if response.hovered(){
                    let scroll=ui.input(|i|i.raw_scroll_delta.y);
                    if scroll.abs()>0.0{
                        changed|=zoom(&mut self.camera,scroll);
                    }
                }
                if changed{self.invalidate();}
            }
        });
    }
    fn schedule(&mut self){
        match self.view{
            ViewMode::Solid=>{
                if self.last_edit.elapsed()>=Duration::from_millis(75){
                    self.start_solid();
                }
            }
            ViewMode::Live=>{
                if self.last_edit.elapsed()>=Duration::from_millis(160){
                    self.start_live();
                }
            },
            ViewMode::Final=>{},
        }
    }
}
fn vector(ui:&mut egui::Ui,p:&mut Vec3)->bool{
    let mut changed=false;
    ui.horizontal_wrapped(|ui|{
        changed|=ui.add(egui::DragValue::new(&mut p.x).speed(0.1).prefix("X ")).changed();
        changed|=ui.add(egui::DragValue::new(&mut p.y).speed(0.1).prefix("Y ")).changed();
        changed|=ui.add(egui::DragValue::new(&mut p.z).speed(0.1).prefix("Z ")).changed();
    });
    changed
}
fn preview_size(settings:RenderSettings,max_width:u32)->(u32,u32){
    let width=settings.width.min(max_width).max(1);
    let aspect=settings.height as f32/settings.width.max(1) as f32;
    let height=(width as f32*aspect).round().clamp(1.0,max_width as f32) as u32;
    (width,height)
}
impl eframe::App for StudioApp {
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame){
        self.receive(ctx);
        self.top(ctx);
        egui::TopBottomPanel::bottom("rv_status").show(ctx,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.label(&self.status);
                ui.separator();
                ui.label(if self.final_busy{"Render final na CPU"}else{"CirclesRust • CPU"});
            });
        });
        self.camera_panel(ctx);
        self.left(ctx);
        self.right(ctx);
        self.center(ctx);
        self.schedule();
        if self.solid_busy||self.live_busy||self.final_busy{
            ctx.request_repaint_after(Duration::from_millis(75));
        }
    }
}
pub fn run()->eframe::Result{
    let options=eframe::NativeOptions{
        viewport:egui::ViewportBuilder::default()
            .with_inner_size([1460.0,910.0])
            .with_min_inner_size([940.0,640.0]),
        ..Default::default()
    };
    eframe::run_native("RV Studio Render | CirclesRust",options,
        Box::new(|cc|Ok(Box::new(StudioApp::new(cc)))))
}
