// SPDX-License-Identifier: Apache-2.0 AND MIT
//! Persistent GPU resources / egui callback adapted from SolidCraft ui-egui/gpu.rs.
//! See ATTRIBUTION.md. Geometry is uploaded once; edits update object matrices.
use std::sync::Arc;
use eframe::{egui,egui_wgpu};
use wgpu::util::DeviceExt;
/// The app and offscreen tests request the same native limits. Older GL adapters
/// can still host egui and use the CPU viewport without vertex storage buffers.
pub fn device_descriptor(adapter:&wgpu::Adapter)->wgpu::DeviceDescriptor<'static>{
    let storage=adapter.get_downlevel_capabilities().flags.contains(wgpu::DownlevelFlags::VERTEX_STORAGE)
        && adapter.limits().max_storage_buffers_per_shader_stage>0;
    let mut limits=wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
    if storage {
        limits.max_storage_buffers_per_shader_stage=1;
        limits.max_storage_buffer_binding_size=adapter.limits().max_storage_buffer_binding_size.min(128*1024*1024);
    }
    wgpu::DeviceDescriptor{label:Some("RV viewport and egui"),required_limits:limits,..Default::default()}
}
pub struct Frame { pub mesh:Arc<Vec<u8>>,pub transforms:Vec<u8>,pub camera:Vec<u8> }
pub struct Renderer {
    pipeline:wgpu::RenderPipeline,layout:wgpu::BindGroupLayout,
    uniform:wgpu::Buffer,objects:wgpu::Buffer,bind:wgpu::BindGroup,
    vertices:wgpu::Buffer,source:Option<Arc<Vec<u8>>>,transforms:Vec<u8>,count:u32,
    pub mesh_uploads:u64,
}
impl Renderer {
    pub fn new(device:&wgpu::Device,format:wgpu::TextureFormat)->Self {
        let layout=device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {label:Some("viewport bindings"),entries:&[
            wgpu::BindGroupLayoutEntry {binding:0,visibility:wgpu::ShaderStages::VERTEX_FRAGMENT,ty:wgpu::BindingType::Buffer {ty:wgpu::BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None},count:None},
            wgpu::BindGroupLayoutEntry {binding:1,visibility:wgpu::ShaderStages::VERTEX,ty:wgpu::BindingType::Buffer {ty:wgpu::BufferBindingType::Storage {read_only:true},has_dynamic_offset:false,min_binding_size:None},count:None},
        ]});
        let uniform=device.create_buffer(&wgpu::BufferDescriptor {label:Some("viewport camera"),size:112,usage:wgpu::BufferUsages::UNIFORM|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let objects=device.create_buffer(&wgpu::BufferDescriptor {label:Some("viewport objects"),size:128,usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let bind=Self::bind(device,&layout,&uniform,&objects);
        let vertices=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:40,usage:wgpu::BufferUsages::VERTEX,mapped_at_creation:false});
        let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("solid viewport"),source:wgpu::ShaderSource::Wgsl(include_str!("viewport.wgsl").into())});
        let pipeline_layout=device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{label:None,bind_group_layouts:&[&layout],push_constant_ranges:&[]});
        let pipeline=device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:Some("solid viewport"),layout:Some(&pipeline_layout),
            vertex:wgpu::VertexState {module:&shader,entry_point:Some("vertex"),compilation_options:Default::default(),buffers:&[wgpu::VertexBufferLayout {array_stride:40,step_mode:wgpu::VertexStepMode::Vertex,attributes:&wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x3,3=>Uint32]}]},
            fragment:Some(wgpu::FragmentState {module:&shader,entry_point:Some("fragment"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState{format,blend:None,write_mask:wgpu::ColorWrites::ALL})]}),
            primitive:wgpu::PrimitiveState {cull_mode:None,..Default::default()},
            depth_stencil:Some(wgpu::DepthStencilState {format:wgpu::TextureFormat::Depth24Plus,depth_write_enabled:true,depth_compare:wgpu::CompareFunction::Less,stencil:Default::default(),bias:Default::default()}),
            multisample:Default::default(),multiview:None,cache:None,
        });
        Self {pipeline,layout,uniform,objects,bind,vertices,source:None,transforms:Vec::new(),count:0,mesh_uploads:0}
    }
    fn bind(d:&wgpu::Device,l:&wgpu::BindGroupLayout,u:&wgpu::Buffer,o:&wgpu::Buffer)->wgpu::BindGroup {
        d.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:l,entries:&[wgpu::BindGroupEntry{binding:0,resource:u.as_entire_binding()},wgpu::BindGroupEntry{binding:1,resource:o.as_entire_binding()}]})
    }
    pub fn prepare(&mut self,d:&wgpu::Device,q:&wgpu::Queue,f:&Frame){
        if !self.source.as_ref().is_some_and(|m|Arc::ptr_eq(m,&f.mesh)){
            if !f.mesh.is_empty(){self.vertices=d.create_buffer_init(&wgpu::util::BufferInitDescriptor {label:Some("cached mesh"),contents:&f.mesh,usage:wgpu::BufferUsages::VERTEX});}
            self.count=(f.mesh.len()/40) as u32;self.source=Some(f.mesh.clone());self.mesh_uploads+=1;
        }
        if self.transforms!=f.transforms {
            if f.transforms.len() as u64>self.objects.size(){
                self.objects=d.create_buffer(&wgpu::BufferDescriptor {label:None,size:f.transforms.len() as u64,usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
                self.bind=Self::bind(d,&self.layout,&self.uniform,&self.objects);
            }
            if !f.transforms.is_empty(){q.write_buffer(&self.objects,0,&f.transforms);}
            self.transforms.clone_from(&f.transforms);
        }
        q.write_buffer(&self.uniform,0,&f.camera);
    }
    pub fn paint(&self,pass:&mut wgpu::RenderPass<'_>){
        pass.set_pipeline(&self.pipeline);pass.set_bind_group(0,&self.bind,&[]);
        pass.set_vertex_buffer(0,self.vertices.slice(..));pass.draw(0..self.count,0..1);
    }
}
pub fn install(rs:&egui_wgpu::RenderState){
    rs.renderer.write().callback_resources.insert(Renderer::new(&rs.device,rs.target_format));
}
pub struct ViewportCallback(pub Frame);
impl egui_wgpu::CallbackTrait for ViewportCallback {
    fn prepare(&self,device:&wgpu::Device,queue:&wgpu::Queue,_screen:&egui_wgpu::ScreenDescriptor,_encoder:&mut wgpu::CommandEncoder,resources:&mut egui_wgpu::CallbackResources)->Vec<wgpu::CommandBuffer>{
        if let Some(r)=resources.get_mut::<Renderer>() {r.prepare(device,queue,&self.0);} Vec::new()
    }
    fn paint(&self,info:egui::PaintCallbackInfo,pass:&mut wgpu::RenderPass<'static>,resources:&egui_wgpu::CallbackResources){
        if let Some(r)=resources.get::<Renderer>(){
            let vp=info.viewport_in_pixels();
            if vp.width_px>0 && vp.height_px>0 {
                pass.set_viewport(vp.left_px as f32,vp.top_px as f32,vp.width_px as f32,vp.height_px as f32,0.0,1.0);
                r.paint(pass);
            }
        }
    }
}
