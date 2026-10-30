// SPDX-License-Identifier: Apache-2.0
//! RV Studio Render native Rust UI with three distinct, functional view modes.
//! Solid viewport draws cached meshes through wgpu. Live preview uses low-sample
//! CPU path tracing on scene edits and progressively refines. Final view keeps
//! the full-resolution image, with a permanent route back to the environment.
//! Path tracing remains CPU; the solid viewport is a separate raster pipeline.
mod theme;
pub mod gpu_view;
use crate::mesh_view;
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
use crate::scene::selection::{pick,frame_object};
use crate::scene::transform::ObjectTransform;
use crate::scene::history::TransformHistory;
use crate::desktop_transform::{EditTool,EditAxis,apply_drag};
use theme::Tokens;
use crate::gizmo::{self,AxisHandle};

#[derive(Clone,Copy)]
struct GizmoDrag { handle:AxisHandle, initial:ObjectTransform, pointer:egui::Pos2, tool:EditTool }

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
    gpu:Option<eframe::egui_wgpu::RenderState>,
    gpu_mesh:Option<(Vec<u32>,Arc<Vec<u8>>)>,
    pending_transform:Option<(usize,ObjectTransform)>,
    defer_transforms:bool,
    viewport_backend:String,
    colors:Tokens,
    options:SceneLoadOptions,
    obj_path:String,mtl_path:String,environment_path:String,output_path:String,
    settings:RenderSettings,camera:CameraSettings,denoise:u8,active_material:usize,
    scene:Option<Arc<LoadedScene>>,
    view:ViewMode,revision:u64,
    selected_object:Option<usize>,
    edit_tool:EditTool,edit_axis:EditAxis,
    nav_tool:NavigationTool,
    history:TransformHistory,
    suppress_gesture:bool,
    gizmo_drag:Option<GizmoDrag>,
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
        Self::from_context(&cc.egui_ctx,cc.wgpu_render_state.clone())
    }
    fn from_context(ctx:&egui::Context,gpu:Option<eframe::egui_wgpu::RenderState>)->Self {
        let gpu=gpu.filter(|rs|rs.device.limits().max_storage_buffers_per_shader_stage>0);
        let viewport_backend=if let Some(rs)=&gpu{
            gpu_view::install(rs);
            format!("Viewport {:?}: {}",rs.adapter.get_info().backend,rs.adapter.get_info().name)
        }else{"Viewport CPU (compatibilidade)".into()};
        let colors=Tokens::palette(true);
        colors.apply(ctx);
        let (sender,receiver)=mpsc::channel();
        Self{
            gpu,gpu_mesh:None,pending_transform:None,defer_transforms:false,viewport_backend,
            colors,options:SceneLoadOptions::studio(),
            obj_path:String::new(),mtl_path:String::new(),environment_path:String::new(),
            output_path:"RV_Studio_Render.png".into(),
            settings:RenderSettings{
                width:960,height:540,samples:24,max_bounces:5,exposure:1.2,seed:17
            },
            camera:CameraSettings::default(),denoise:2,active_material:0,
            scene:load_scene(&SceneLoadOptions::studio()).ok().map(Arc::new),
            view:ViewMode::Solid,revision:1,
            selected_object:None,
            edit_tool:EditTool::Select,edit_axis:EditAxis::Free,
            nav_tool:NavigationTool::Orbit,
            history:TransformHistory::default(),suppress_gesture:false,gizmo_drag:None,
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
                self.gpu_mesh=None;self.pending_transform=None;
                self.history=TransformHistory::default();
                self.gizmo_drag=None;
                self.active_material=0;
                self.selected_object=None;
                self.final_image=None;
                self.view=ViewMode::Solid;
                self.invalidate();
            }
            Err(e)=>self.status=format!("Não foi possível importar a cena: {e}"),
        }
    }
    fn select_object(&mut self,id:usize){
        self.finish_pending_transform();
        self.gizmo_drag=None;
        self.history.finish();
        if let Some(loaded)=&self.scene{
            if id>=loaded.scene.object_names.len(){return;}
            if let Some((triangle,_))=loaded.scene.triangle_objects.iter()
                .enumerate().find(|(_,object)|**object==id){
                self.active_material=loaded.scene.triangle_materials[triangle];
            }
            self.selected_object=Some(id);
            self.status=format!("Objeto selecionado: {} (pressione F para enquadrar).",
                loaded.scene.object_names[id]);
            self.invalidate();
        }
    }
    fn focus_selection(&mut self){
        if let (Some(scene),Some(id))=(&self.scene,self.selected_object){
            let aspect=self.settings.width.max(1) as f32/self.settings.height.max(1) as f32;
            if frame_object(&scene.scene,&mut self.camera,id,aspect){
                self.invalidate();
            }
        }
    }
    fn current_transform(&self,id:usize)->Option<ObjectTransform>{
        self.pending_transform.filter(|(i,_)|*i==id).map(|(_,t)|t)
            .or_else(||self.scene.as_ref()?.scene.object_transforms.get(id).copied())
    }
    fn finish_pending_transform(&mut self){
        if let Some((id,t))=self.pending_transform.take(){
            if self.selected_object==Some(id){
                let deferred=self.defer_transforms;self.defer_transforms=false;
                self.commit_transform(t);self.defer_transforms=deferred;
            }
        }
    }
    fn commit_transform(&mut self,value:ObjectTransform){
        if self.suppress_gesture{return;}
        let Some(id)=self.selected_object else{return;};
        if !value.valid(){return;}
        if self.defer_transforms {
            if self.current_transform(id)!=Some(value){self.pending_transform=Some((id,value));self.invalidate();}
            return;
        }
        let Some(scene)=self.scene.as_mut() else{return;};
        let result=self.history.apply(&mut Arc::make_mut(scene).scene,id,value);
        match result{
            Ok(false)=>{},
            Ok(true)=>{
                self.status="Geometria e BVH atualizadas — prévia e render usarão nova transformação.".into();
                self.invalidate();
            }
            Err(err)=>self.status=format!("Transformação recusada: {err}"),
        }
    }
    fn reset_transform(&mut self){
        self.finish_pending_transform();
        self.history.finish();
        self.commit_transform(ObjectTransform::default());
        self.history.finish();
    }
    fn history_action(&mut self,redo:bool,cancel:bool){
        if cancel && self.pending_transform.take().is_some(){
            self.invalidate();self.status="Transformação cancelada.".into();return;
        }
        self.finish_pending_transform();
        let Some(scene)=self.scene.as_mut() else{return;};
        let scene=&mut Arc::make_mut(scene).scene;
        let result=if cancel{self.history.cancel(scene)}
            else if redo{self.history.redo(scene)}else{self.history.undo(scene)};
        match result{
            Ok(Some(id))=>{
                self.select_object(id);
                self.status=if cancel{"Transformação cancelada."}
                    else if redo{"Transformação refeita."}else{"Transformação desfeita."}.into();
            }
            Ok(None)=>{},
            Err(err)=>self.status=format!("Não foi possível restaurar: {err}"),
        }
    }
    fn launch_final(&mut self){
        self.finish_pending_transform();
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
        let selected=self.selected_object;
        let tint=self.colors.selection_tint;
        self.solid_busy=true;
        std::thread::spawn(move||{
            let result=preview::solid_preview_cancellable_highlight(
                &snapshot.scene,camera,w,h,&epoch,revision,
                preview::SelectionHighlight{object_id:selected,tint}
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
                    let mut clicked_object=None;
                    if let Some(scene)=&self.scene{
                        ui.separator();
                        ui.label(&scene.source_label);
                        ui.label(format!("{} triângulos • {} objetos",
                            scene.scene.bvh.stats().triangles,
                            scene.scene.object_names.len()));
                        for (id,name) in scene.scene.object_names.iter().enumerate(){
                            if ui.selectable_label(
                                self.selected_object==Some(id),name
                            ).clicked(){clicked_object=Some(id);}
                        }
                    }
                    if let Some(id)=clicked_object{self.select_object(id);}
                    if ui.add_enabled(self.selected_object.is_some(),
                        egui::Button::new("Enquadrar selecionado (F)")).clicked(){
                        self.focus_selection();
                    }
                    ui.label("G = mover • R = girar • S = escala • X/Y/Z = eixo");
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
                    egui::ComboBox::from_label("Material")
                        .selected_text(&loaded.material_names[id])
                        .show_ui(ui,|ui|{
                            for (i,name) in loaded.material_names.iter().enumerate(){
                                ui.selectable_value(&mut self.active_material,i,name);
                            }
                        });
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
                    ui.separator();
                    ui.heading("Transformação do objeto");
                    ui.horizontal_wrapped(|ui|{
                        if ui.add_enabled(self.history.can_undo(),egui::Button::new("Desfazer")).clicked(){
                            self.history_action(false,false);
                        }
                        if ui.add_enabled(self.history.can_redo(),egui::Button::new("Refazer")).clicked(){
                            self.history_action(true,false);
                        }
                    });
                    if let Some(id)=self.selected_object{
                        if let Some(mut t)=self.current_transform(id){
                            let mut changed=false;
                            ui.label("Posição (m)");
                            changed|=vector(ui,&mut t.translation);
                            ui.label("Rotação XYZ (graus)");
                            changed|=vector(ui,&mut t.rotation_degrees);
                            ui.label("Escala XYZ");
                            changed|=vector(ui,&mut t.scale);
                            if changed{self.commit_transform(t);}
                            if ui.button("Restaurar transformação").clicked(){
                                self.reset_transform();
                            }
                        }
                    }else{
                        ui.label("Selecione um objeto no viewport ou no painel esquerdo.");
                    }
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
    fn gpu_frame(&mut self,aspect:f32)->Option<gpu_view::Frame>{
        let rs=self.gpu.as_ref()?;
        let scene=&self.scene.as_ref()?.scene;
        let limits=rs.device.limits();
        let bytes=scene.base_triangles.len().checked_mul(120)?;
        if bytes as u64>limits.max_buffer_size || bytes/40>u32::MAX as usize
            || scene.object_names.len().saturating_mul(128)>limits.max_storage_buffer_binding_size as usize{
            self.gpu=None;self.defer_transforms=false;
            self.finish_pending_transform();
            self.viewport_backend="Viewport CPU: malha excede limites do adaptador".into();
            return None;
        }
        let key:Vec<u32>=scene.materials.iter().flat_map(|m|[m.base_color.x.to_bits(),m.base_color.y.to_bits(),m.base_color.z.to_bits()]).collect();
        if !self.gpu_mesh.as_ref().is_some_and(|(old,_)|*old==key){
            self.gpu_mesh=Some((key,Arc::new(mesh_view::mesh_bytes(scene))));
        }
        Some(gpu_view::Frame{
            mesh:self.gpu_mesh.as_ref()?.1.clone(),
            transforms:mesh_view::transform_bytes(scene,self.pending_transform),
            camera:mesh_view::camera_bytes(self.camera,aspect,scene.light.center,self.selected_object,self.colors.selection_tint,rs.target_format.is_srgb())?,
        })
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
                    ui.label("Clique: selecionar • F: enquadrar • G/R/S: transformação");
                });
            }
            if self.view!=ViewMode::Final{
                ui.horizontal_wrapped(|ui|{
                    ui.label("Ferramenta");
                    ui.selectable_value(&mut self.edit_tool,EditTool::Select,"Selecionar");
                    ui.selectable_value(&mut self.edit_tool,EditTool::Move,"Mover (G)");
                    ui.selectable_value(&mut self.edit_tool,EditTool::Rotate,"Girar (R)");
                    ui.selectable_value(&mut self.edit_tool,EditTool::Scale,"Escala (S)");
                    ui.label("Eixo");
                    for (axis,label) in [
                        (EditAxis::Free,"Livre"),(EditAxis::X,"X"),
                        (EditAxis::Y,"Y"),(EditAxis::Z,"Z")
                    ]{
                        ui.selectable_value(&mut self.edit_axis,axis,label);
                    }
                });
            }
            if self.view==ViewMode::Final {
                ui.label("Arquivo final");
                ui.add(egui::TextEdit::singleline(&mut self.output_path)
                    .desired_width(f32::INFINITY));
                ui.add(egui::Slider::new(&mut self.denoise,0..=5).text("Denoise"));
            }
            let space=ui.available_size();
            let (rect,response)=ui.allocate_exact_size(space,egui::Sense::click_and_drag());
            let painter=ui.painter_at(rect);
            painter.rect_filled(rect,0.0,self.colors.canvas);
            let aspect=self.settings.width.max(1) as f32/self.settings.height.max(1) as f32;
            let use_gpu=self.gpu.is_some() && (self.view==ViewMode::Solid || (self.view==ViewMode::Live && (self.pending_transform.is_some() || self.last_edit.elapsed()<Duration::from_millis(160))));
            let gpu_frame=if use_gpu{self.gpu_frame(aspect)}else{None};
            let texture=match self.view {
                ViewMode::Solid=>self.solid.as_ref(),
                ViewMode::Live=>self.live.as_ref(),
                ViewMode::Final=>self.final_image.as_ref(),
            };
            let mut pick_image=None;
            if let Some(frame)=gpu_frame{
                let width=rect.width().min(rect.height()*aspect).max(1.0);
                let image_rect=egui::Rect::from_center_size(rect.center(),egui::vec2(width,width/aspect));
                pick_image=Some((image_rect,aspect));
                painter.add(eframe::egui_wgpu::Callback::new_paint_callback(image_rect,gpu_view::ViewportCallback(frame)));
            }else if let Some(texture)=texture{
                let original=texture.size_vec2();
                let scale=(rect.width()/original.x).min(rect.height()/original.y).max(0.01);
                let width=original.x*scale;
                let height=original.y*scale;
                let top_left=egui::pos2(rect.center().x-width*0.5,rect.center().y-height*0.5);
                let image_rect=egui::Rect::from_min_size(top_left,egui::vec2(width,height));
                pick_image=Some((image_rect,original.x/original.y));
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
                let mut hovered_handle=None;
                if matches!(self.edit_tool,EditTool::Move|EditTool::Scale){
                    if let (Some((image_rect,_)),Some(id),Some(loaded))=(
                        pick_image,self.selected_object,self.scene.as_ref()
                    ){
                        // Use the true transform pivot, not the rotated world AABB center.
                        let pivot=loaded.scene.object_pivots[id]+self.current_transform(id).unwrap_or_default().translation;
                        let handles=gizmo::handles(self.camera,pivot,
                            [image_rect.width(),image_rect.height()],self.colors.handle_length);
                        if let Some(pointer)=response.hover_pos().filter(|p|image_rect.contains(*p)){
                            hovered_handle=gizmo::hit_test(&handles,
                                [pointer.x-image_rect.left(),pointer.y-image_rect.top()],self.colors.handle_hit_radius);
                        }
                        let axis_painter=ui.painter_at(image_rect.intersect(rect));
                        for handle in handles{
                            let from=image_rect.min+egui::vec2(handle.start[0],handle.start[1]);
                            let to=image_rect.min+egui::vec2(handle.end[0],handle.end[1]);
                            let active=self.gizmo_drag.map(|d|d.handle.axis)==Some(handle.axis)
                                ||hovered_handle.map(|h|h.axis)==Some(handle.axis);
                            let color=if active{self.colors.accent}else{match handle.axis{
                                EditAxis::X=>self.colors.axis_x,EditAxis::Y=>self.colors.axis_y,
                                _=>self.colors.axis_z,
                            }};
                            axis_painter.line_segment([from,to],egui::Stroke::new(self.colors.axis_width,color));
                            if self.edit_tool==EditTool::Scale{
                                axis_painter.rect_filled(egui::Rect::from_center_size(to,
                                    egui::Vec2::splat(self.colors.handle_radius*2.0)),0.0,color);
                            }else{axis_painter.circle_filled(to,self.colors.handle_radius,color);}
                            let label=match handle.axis{EditAxis::X=>"X",EditAxis::Y=>"Y",_=>"Z"};
                            axis_painter.text(to+egui::vec2(self.colors.gap,-self.colors.gap),
                                egui::Align2::LEFT_BOTTOM,label,egui::FontId::proportional(self.colors.brand_size),color);
                        }
                    }
                }
                if hovered_handle.is_some(){ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);}
                if ui.input(|i|i.pointer.primary_pressed()) && !self.suppress_gesture{
                    if let (Some(handle),Some(pointer),Some(id),Some(_scene))=(hovered_handle,
                        response.interact_pointer_pos(),self.selected_object,self.scene.as_ref()){
                        self.history.finish();
                        self.gizmo_drag=Some(GizmoDrag{handle,initial:self.current_transform(id).unwrap_or_default(),
                            pointer,tool:self.edit_tool});
                        self.edit_axis=handle.axis;
                    }
                }
                if let Some(gesture)=self.gizmo_drag{
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    if !self.suppress_gesture{
                        if let Some(pointer)=ui.input(|i|i.pointer.interact_pos()){
                            let delta=pointer-gesture.pointer;
                            if let Some(next)=gizmo::drag(gesture.initial,gesture.tool,gesture.handle,[delta.x,delta.y]){
                                self.commit_transform(next);
                            }
                        }
                    }
                }
                if self.gizmo_drag.is_none() && response.hovered() && !ui.ctx().wants_keyboard_input() && !ui.input(|i|i.modifiers.command){
                    if ui.input(|i|i.key_pressed(egui::Key::G)){self.edit_tool=EditTool::Move;}
                    if ui.input(|i|i.key_pressed(egui::Key::R)){self.edit_tool=EditTool::Rotate;}
                    if ui.input(|i|i.key_pressed(egui::Key::S)){self.edit_tool=EditTool::Scale;}
                    if ui.input(|i|i.key_pressed(egui::Key::Escape)){self.edit_tool=EditTool::Select;}
                    if ui.input(|i|i.key_pressed(egui::Key::X)){self.edit_axis=EditAxis::X;}
                    if ui.input(|i|i.key_pressed(egui::Key::Y)){self.edit_axis=EditAxis::Y;}
                    if ui.input(|i|i.key_pressed(egui::Key::Z)){self.edit_axis=EditAxis::Z;}
                }
                if response.clicked_by(egui::PointerButton::Primary) && hovered_handle.is_none() && self.gizmo_drag.is_none() {
                    if let (Some((image_rect,aspect)),Some(pointer),Some(loaded))=(
                        pick_image,response.interact_pointer_pos(),self.scene.as_ref()
                    ){
                        if image_rect.contains(pointer){
                            let u=(pointer.x-image_rect.left())/image_rect.width();
                            let v=(pointer.y-image_rect.top())/image_rect.height();
                            if let Some(hit)=pick(&loaded.scene,self.camera,u,v,aspect){
                                self.select_object(hit.object_id);
                            }
                        }
                    }
                }
                if self.gizmo_drag.is_none() && response.hovered() && ui.input(|i|i.key_pressed(egui::Key::F)) {
                    self.focus_selection();
                }
                let mut changed=false;
                let using_middle=response.dragged_by(egui::PointerButton::Middle);
                let using_primary=response.dragged_by(egui::PointerButton::Primary);
                let shift=ui.input(|i|i.modifiers.shift);
                if (using_middle||using_primary) && !self.suppress_gesture && self.gizmo_drag.is_none(){
                    let delta=ui.input(|i|i.pointer.delta());
                    if delta.length_sq()>0.0{
                        let editing=using_primary && self.selected_object.is_some()
                            && self.edit_tool!=EditTool::Select;
                        if editing{
                            if let Some(current)=self.selected_object.and_then(|id|
                                self.current_transform(id)
                            ){
                                if let Some(next)=apply_drag(
                                    current,self.edit_tool,self.edit_axis,
                                    delta.x,delta.y,self.camera,rect.height()
                                ){
                                    self.commit_transform(next);
                                }
                            }
                        }else{
                            let do_pan=if using_middle{shift}
                                else{self.nav_tool==NavigationTool::Pan};
                            changed|=if do_pan{
                                pan(&mut self.camera,delta.x,delta.y,rect.height())
                            }else{
                                orbit(&mut self.camera,delta.x,delta.y)
                            };
                        }
                    }
                }
                if response.hovered() && self.gizmo_drag.is_none(){
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
                if self.gpu.is_none() && self.last_edit.elapsed()>=Duration::from_millis(75){
                    self.start_solid();
                }
            }
            ViewMode::Live=>{
                if self.pending_transform.is_none() && self.last_edit.elapsed()>=Duration::from_millis(160){
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
        let before=self.revision;
        self.receive(ctx);
        let pointer_down=ctx.input(|i|i.pointer.primary_down());
        self.defer_transforms=pointer_down && self.gpu.is_some();
        if !pointer_down{self.suppress_gesture=false;}
        // Escape cancels a gesture even if its pointer has left the viewport.
        if (self.history.is_editing() || self.gizmo_drag.is_some() || self.pending_transform.is_some()) && ctx.input(|i|i.key_pressed(egui::Key::Escape)){
            self.history_action(false,true);
            self.suppress_gesture=pointer_down;
            self.gizmo_drag=None;
        }
        if !ctx.wants_keyboard_input() && !pointer_down{
            let redo=ctx.input_mut(|i|i.consume_key(egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),egui::Key::Z)
                ||i.consume_key(egui::Modifiers::COMMAND,egui::Key::Y));
            let undo=ctx.input_mut(|i|i.consume_key(egui::Modifiers::COMMAND,egui::Key::Z));
            if redo{self.history_action(true,false);}
            else if undo{self.history_action(false,false);}
        }
        self.top(ctx);
        egui::TopBottomPanel::bottom("rv_status").show(ctx,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.label(&self.status);
                ui.separator();
                ui.label(&self.viewport_backend);
            });
        });
        self.camera_panel(ctx);
        self.left(ctx);
        self.right(ctx);
        self.center(ctx);
        if !pointer_down{self.finish_pending_transform();self.history.finish();self.gizmo_drag=None;}
        self.schedule();
        if before!=self.revision{ctx.request_repaint();}
        if (self.view==ViewMode::Live && self.live_pass<3 && self.pending_transform.is_none())
            || (self.view==ViewMode::Solid && self.gpu.is_none() && self.solid_revision!=Some(self.revision)){
            ctx.request_repaint_after(Duration::from_millis(20));
        }
        if self.solid_busy||self.live_busy||self.final_busy{
            ctx.request_repaint_after(Duration::from_millis(75));
        }
    }
}
pub fn run()->eframe::Result{
    let cpu=std::env::args().any(|s|s=="--cpu-viewport");
    let setup=eframe::egui_wgpu::WgpuSetupCreateNew{
        device_descriptor:Arc::new(gpu_view::device_descriptor),..Default::default()
    };
    let options=eframe::NativeOptions{
        wgpu_options:eframe::egui_wgpu::WgpuConfiguration{wgpu_setup:setup.into(),..Default::default()},
        renderer:if cpu{eframe::Renderer::Glow}else{eframe::Renderer::Wgpu},
        depth_buffer:24,multisampling:0,
        viewport:egui::ViewportBuilder::default()
            .with_inner_size([1460.0,910.0])
            .with_min_inner_size([940.0,640.0]),
        ..Default::default()
    };
    eframe::run_native("RV Studio Render | CirclesRust",options,
        Box::new(|cc|Ok(Box::new(StudioApp::new(cc)))))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thousand_drag_updates_keep_scene_and_bvh_until_release(){
        let mut app=StudioApp::from_context(&egui::Context::default(),None);
        app.select_object(0);
        let snapshot=app.scene.as_ref().unwrap().clone();
        app.defer_transforms=true;
        for i in 1..=1000 {
            let mut t=ObjectTransform::default();t.translation.x=i as f32*0.001;
            app.commit_transform(t);
            assert!(Arc::ptr_eq(app.scene.as_ref().unwrap(),&snapshot));
            assert!(!app.history.can_undo());
        }
        app.finish_pending_transform();app.history.finish();
        assert_eq!(app.scene.as_ref().unwrap().scene.object_transforms[0].translation.x,1.0);
        app.history_action(false,false);
        assert_eq!(app.scene.as_ref().unwrap().scene.object_transforms[0],ObjectTransform::default());
        app.history_action(true,false);
        assert_eq!(app.scene.as_ref().unwrap().scene.object_transforms[0].translation.x,1.0);
    }
    #[test]
    fn cancelling_preview_preserves_scene_and_redo(){
        let mut app=StudioApp::from_context(&egui::Context::default(),None);
        app.select_object(0);
        let mut t=ObjectTransform::default();t.translation.x=1.0;
        app.commit_transform(t);app.history.finish();app.history_action(false,false);
        let snapshot=app.scene.as_ref().unwrap().clone();
        app.defer_transforms=true;t.translation.x=2.0;app.commit_transform(t);
        app.history_action(false,true);
        assert!(Arc::ptr_eq(app.scene.as_ref().unwrap(),&snapshot));
        assert!(app.history.can_redo());assert!(app.pending_transform.is_none());
    }
}
