// SPDX-License-Identifier: Apache-2.0
//! Native desktop shell bound directly to the Rust CPU path tracer.
//! Preview shows real rendered pixels, NOT a simulated live 3D viewport.
mod theme;
use std::{path::PathBuf,sync::{Arc,mpsc},time::Duration};
use eframe::egui;
use crate::integrator::{CameraSettings,RenderSettings,render_rgb_denoised_with_camera};
use crate::scene::loader::{load_scene,SceneLoadOptions,LoadedScene};
use crate::film::save_image;
use crate::util::math::Vec3;
use theme::Tokens;

type Finished=Result<(Vec<u8>,u32,u32,String),String>;
struct StudioApp {
    colors:Tokens,
    options:SceneLoadOptions,
    obj_path:String, mtl_path:String, environment_path:String,
    output_path:String,
    settings:RenderSettings, camera:CameraSettings, denoise:u8,
    active_material:usize,
    scene:Option<Arc<LoadedScene>>,
    image:Option<egui::TextureHandle>,
    waiting:Option<mpsc::Receiver<Finished>>,
    status:String,
}
impl StudioApp {
    fn new(cc:&eframe::CreationContext<'_>)->Self{
        let colors=Tokens::palette(true);
        colors.apply(&cc.egui_ctx);
        Self {
            colors,options:SceneLoadOptions::studio(),
            obj_path:String::new(),mtl_path:String::new(),environment_path:String::new(),
            output_path:"RV_Studio_Render.png".into(),
            settings:RenderSettings{
                width:960,height:540,samples:24,max_bounces:5,exposure:1.2,seed:17
            },
            camera:CameraSettings::default(),denoise:2,active_material:0,
            scene:load_scene(&SceneLoadOptions::studio()).ok().map(Arc::new),
            image:None,waiting:None,status:"Cena carregada. GPU ainda não integrada.".into(),
        }
    }
    fn busy(&self)->bool{self.waiting.is_some()}
    fn load(&mut self){
        if self.busy(){return;}
        let obj=self.obj_path.trim();
        let mtl=self.mtl_path.trim();
        let env=self.environment_path.trim();
        self.options.obj=(!obj.is_empty()).then(||PathBuf::from(obj));
        self.options.mtl=(!mtl.is_empty()).then(||PathBuf::from(mtl));
        self.options.environment=(!env.is_empty()).then(||PathBuf::from(env));
        match load_scene(&self.options) {
            Ok(loaded)=>{
                self.status=format!(
                    "{} | {} triângulos | {} materiais",
                    loaded.source_label,loaded.scene.bvh.stats().triangles,
                    loaded.material_names.len()
                );
                self.scene=Some(Arc::new(loaded));
                self.image=None;self.active_material=0;
            }
            Err(err)=>self.status=format!("Erro ao carregar: {err}"),
        }
    }
    fn launch(&mut self){
        if self.busy(){return;}
        let Some(scene)=self.scene.as_ref().cloned() else{
            self.status="Nenhuma cena para renderizar.".into();return;
        };
        if let Err(e)=self.settings.validate(){self.status=e.into();return;}
        if let Err(e)=self.camera.validate(){self.status=e.into();return;}
        if self.denoise>5{self.status="Denoise deve estar entre 0 e 5".into();return;}
        let settings=self.settings;
        let camera=self.camera;
        let denoise=self.denoise;
        let output=self.output_path.clone();
        let (tx,rx)=mpsc::channel();
        self.waiting=Some(rx);
        self.status=format!("Renderizando {} × {}, {} amostras na CPU...",
            settings.width,settings.height,settings.samples);
        std::thread::spawn(move||{
            let result=(||->Finished{
                let pixels=render_rgb_denoised_with_camera(
                    &scene.scene,settings,camera,denoise
                ).map_err(str::to_owned)?;
                save_image(&output,&pixels,settings.width,settings.height)
                    .map_err(|e|format!("Falha ao salvar: {e}"))?;
                Ok((pixels,settings.width,settings.height,output))
            })();
            let _=tx.send(result);
        });
    }
    fn poll_result(&mut self,ctx:&egui::Context){
        let polled=self.waiting.as_ref().map(mpsc::Receiver::try_recv);
        match polled{
            Some(Ok(result))=>{
                self.waiting=None;
                match result {
                    Ok((pixels,width,height,path))=>{
                        let image=egui::ColorImage::from_rgb(
                            [width as usize,height as usize],&pixels
                        );
                        self.image=Some(ctx.load_texture(
                            "CirclesRust Render",image,egui::TextureOptions::LINEAR
                        ));
                        self.status=format!("Render concluído: {path} ({}×{}).",width,height);
                    }
                    Err(error)=>self.status=format!("Falha: {error}"),
                }
            }
            Some(Err(mpsc::TryRecvError::Disconnected))=>{
                self.waiting=None;
                self.status="O processo de render encerrou sem imagem.".into();
            }
            _=>{},
        }
    }
    fn top(&mut self,ctx:&egui::Context){
        egui::TopBottomPanel::top("rv_topbar")
            .exact_height(self.colors.top_height).show(ctx,|ui|{
                ui.horizontal_centered(|ui|{
                    ui.heading(egui::RichText::new("RV STUDIO RENDER")
                        .size(self.colors.brand_size));
                    ui.separator();
                    ui.label("CirclesRust • CPU");
                    ui.label("Largura");
                    ui.add(egui::DragValue::new(&mut self.settings.width).range(1..=4096));
                    ui.label("Altura");
                    ui.add(egui::DragValue::new(&mut self.settings.height).range(1..=4096));
                    ui.label("Amostras");
                    ui.add(egui::DragValue::new(&mut self.settings.samples).range(1..=1024));
                    if ui.add_enabled(!self.busy(),egui::Button::new(
                        egui::RichText::new("RENDERIZAR").strong().color(self.colors.accent)
                    )).clicked(){self.launch();}
                    if ui.button(if self.colors.dark{"Tema claro"}else{"Tema escuro"}).clicked(){
                        self.colors=Tokens::palette(!self.colors.dark);
                        self.colors.apply(ctx);
                    }
                });
            });
    }
    fn left(&mut self,ctx:&egui::Context){
        egui::SidePanel::left("rv_left")
            .resizable(true).default_width(self.colors.left_width).show(ctx,|ui|{
                egui::ScrollArea::vertical().show(ui,|ui|{
                    ui.add_space(self.colors.padding);
                    ui.heading("Ambiente / Global");
                    ui.label("HDR / PNG / JPEG panorâmico");
                    ui.text_edit_singleline(&mut self.environment_path);
                    ui.add(egui::Slider::new(
                        &mut self.options.environment_strength,0.0..=15.0
                    ).text("Intensidade"));
                    if ui.add_enabled(!self.busy(),egui::Button::new("Aplicar ambiente")).clicked(){
                        self.load();
                    }
                    if let Some(scene)=self.scene.as_mut().and_then(Arc::get_mut){
                        ui.separator();
                        ui.label("Luz de área");
                        ui.add(egui::Slider::new(&mut scene.scene.light.center.z,1.0..=12.0)
                            .text("Altura"));
                        ui.add(egui::Slider::new(&mut scene.scene.light.radiance.x,0.0..=20.0)
                            .text("Luz R"));
                        ui.add(egui::Slider::new(&mut scene.scene.light.radiance.y,0.0..=20.0)
                            .text("Luz G"));
                        ui.add(egui::Slider::new(&mut scene.scene.light.radiance.z,0.0..=20.0)
                            .text("Luz B"));
                    }
                    ui.add_space(self.colors.gap);
                    ui.separator();
                    ui.heading("Objetos da cena");
                    ui.label("Arquivo OBJ");
                    ui.text_edit_singleline(&mut self.obj_path);
                    ui.label("Biblioteca MTL (opcional)");
                    ui.text_edit_singleline(&mut self.mtl_path);
                    if ui.add_enabled(!self.busy(),egui::Button::new(
                        "Importar / Recarregar OBJ"
                    )).clicked(){self.load();}
                    ui.separator();
                    if let Some(scene)=&self.scene{
                        ui.label(&scene.source_label);
                        ui.label(format!("{} faces trianguladas",scene.scene.bvh.stats().triangles));
                        for (i,name) in scene.material_names.iter().enumerate(){
                            ui.selectable_value(&mut self.active_material,i,name);
                        }
                    }
                });
            });
    }
    fn right(&mut self,ctx:&egui::Context){
        egui::SidePanel::right("rv_materials")
            .resizable(true).default_width(self.colors.right_width).show(ctx,|ui|{
                egui::ScrollArea::vertical().show(ui,|ui|{
                    ui.add_space(self.colors.padding);
                    ui.heading("Materiais");
                    let Some(scene)=&self.scene else{
                        ui.label("Carregue uma cena.");return;
                    };
                    let index=self.active_material.min(scene.scene.materials.len()-1);
                    ui.label(&scene.material_names[index]);
                    if let Some(scene)=self.scene.as_mut().and_then(Arc::get_mut){
                        let material=&mut scene.scene.materials[index];
                        ui.label("Cor base");
                        let mut color=[
                            material.base_color.x,material.base_color.y,material.base_color.z
                        ];
                        if ui.color_edit_button_rgb(&mut color).changed(){
                            material.base_color=Vec3::new(color[0],color[1],color[2]);
                        }
                        ui.add(egui::Slider::new(&mut material.roughness,0.045..=1.0)
                            .text("Rugosidade"));
                        ui.add(egui::Slider::new(&mut material.metallic,0.0..=1.0)
                            .text("Metalicidade"));
                    }else{
                        ui.label("Edição bloqueada durante a renderização.");
                    }
                    ui.separator();
                    ui.label("Os mapas PBR/UV são carregados do arquivo MTL.");
                    ui.label("Shaders Principled completos: pendentes.");
                });
            });
    }
    fn camera(&mut self,ctx:&egui::Context){
        egui::TopBottomPanel::bottom("rv_camera")
            .resizable(true).default_height(self.colors.camera_height).show(ctx,|ui|{
                ui.heading("Câmera");
                ui.horizontal(|ui|{
                    ui.vertical(|ui|{
                        ui.label("Posição");
                        vector(ui,&mut self.camera.origin);
                    });
                    ui.separator();
                    ui.vertical(|ui|{
                        ui.label("Alvo");
                        vector(ui,&mut self.camera.target);
                    });
                    ui.separator();
                    ui.vertical(|ui|{
                        ui.label("Campo de visão");
                        ui.add(egui::Slider::new(
                            &mut self.camera.fov_degrees,5.0..=150.0
                        ).text("FOV"));
                        ui.add(egui::Slider::new(
                            &mut self.settings.exposure,0.1..=4.0
                        ).text("Exposição"));
                    });
                });
            });
    }
    fn center(&mut self,ctx:&egui::Context){
        egui::CentralPanel::default().show(ctx,|ui|{
            ui.heading("Viewport | Imagem calculada");
            ui.label(egui::RichText::new(
                "Prévia do render final. Visualização 3D interativa ainda não implementada."
            ).color(self.colors.muted));
            ui.separator();
            ui.label("Salvar imagem (PNG ou BMP)");
            ui.text_edit_singleline(&mut self.output_path);
            ui.horizontal(|ui|{
                ui.label("Ressaltos");
                ui.add(egui::Slider::new(&mut self.settings.max_bounces,1..=32));
                ui.label("Denoise");
                ui.add(egui::Slider::new(&mut self.denoise,0..=5));
            });
            ui.separator();
            egui::Frame::default().fill(self.colors.canvas).show(ui,|ui|{
                ui.set_min_size(ui.available_size());
                if let Some(texture)=&self.image{
                    let space=ui.available_size();
                    let dims=texture.size_vec2();
                    let scale=(space.x/dims.x).min(space.y/dims.y).max(0.01);
                    ui.add(egui::Image::new((texture.id(),dims*scale)));
                }else{
                    ui.centered_and_justified(|ui|{
                        ui.label("Clique em RENDERIZAR para gerar pixels reais pelo motor Rust.");
                    });
                }
            });
        });
    }
}
fn vector(ui:&mut egui::Ui,v:&mut Vec3){
    ui.horizontal(|ui|{
        ui.add(egui::DragValue::new(&mut v.x).speed(0.1).prefix("X "));
        ui.add(egui::DragValue::new(&mut v.y).speed(0.1).prefix("Y "));
        ui.add(egui::DragValue::new(&mut v.z).speed(0.1).prefix("Z "));
    });
}
impl eframe::App for StudioApp {
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame){
        self.poll_result(ctx);
        self.top(ctx);
        egui::TopBottomPanel::bottom("rv_status").show(ctx,|ui|{
            ui.horizontal(|ui|{
                ui.label(&self.status);
                ui.separator();
                ui.label("Rust CPU • GPU em desenvolvimento");
            });
        });
        self.camera(ctx);
        self.left(ctx);
        self.right(ctx);
        self.center(ctx);
        if self.busy(){ctx.request_repaint_after(Duration::from_millis(100));}
    }
}
pub fn run()->eframe::Result{
    let options=eframe::NativeOptions{
        viewport:egui::ViewportBuilder::default()
            .with_inner_size([1450.0,900.0])
            .with_min_inner_size([1020.0,670.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RV Studio Render | CirclesRust",options,
        Box::new(|cc|Ok(Box::new(StudioApp::new(cc))))
    )
}
