#![cfg(feature="desktop-ui")]
use circlesrust::{desktop::gpu_view::{Frame,Renderer},integrator::CameraSettings,mesh_view,scene::{loader::{load_scene,SceneLoadOptions},transform::ObjectTransform},util::math::Vec3};
use std::sync::Arc;
#[test]
fn render_depth_selection_camera_and_transform_without_mesh_uploads(){
    let instance=wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter=match pollster::block_on(instance.request_adapter(&Default::default())) {
        Ok(a)=>a,Err(e)=>{assert!(std::env::var_os("RV_REQUIRE_GPU").is_none(),"GPU required: {e}");eprintln!("SKIP: no adapter: {e}");return;}
    };
    eprintln!("Adapter: {:?}",adapter.get_info());
    let (device,queue)=pollster::block_on(adapter.request_device(&circlesrust::desktop::gpu_view::device_descriptor(&adapter))).unwrap();
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer=Renderer::new(&device,wgpu::TextureFormat::Rgba8Unorm);
    let scene=load_scene(&SceneLoadOptions::studio()).unwrap().scene;
    let camera=CameraSettings::default();
    let uniform=|c,selected|mesh_view::camera_bytes(c,1.0,scene.light.center,selected,Vec3::new(0.95,0.57,0.11),false).unwrap();
    let mut frame=Frame{mesh:Arc::new(mesh_view::mesh_bytes(&scene)),transforms:mesh_view::transform_bytes(&scene,None),camera:uniform(camera,None)};
    let draw=|renderer:&mut Renderer,frame:&Frame|{
        renderer.prepare(&device,&queue,frame);
        let tex=|format,usage|device.create_texture(&wgpu::TextureDescriptor{label:None,size:wgpu::Extent3d{width:256,height:256,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format,usage,view_formats:&[]});
        let color=tex(wgpu::TextureFormat::Rgba8Unorm,wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::COPY_SRC);
        let depth=tex(wgpu::TextureFormat::Depth24Plus,wgpu::TextureUsages::RENDER_ATTACHMENT);
        let cv=color.create_view(&Default::default());let dv=depth.create_view(&Default::default());
        let mut encoder=device.create_command_encoder(&Default::default());
        {
            let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor {label:None,color_attachments:&[Some(wgpu::RenderPassColorAttachment {view:&cv,resolve_target:None,ops:wgpu::Operations {load:wgpu::LoadOp::Clear(wgpu::Color::BLACK),store:wgpu::StoreOp::Store}})],depth_stencil_attachment:Some(wgpu::RenderPassDepthStencilAttachment {view:&dv,depth_ops:Some(wgpu::Operations{load:wgpu::LoadOp::Clear(1.0),store:wgpu::StoreOp::Store}),stencil_ops:None}),timestamp_writes:None,occlusion_query_set:None});
            renderer.paint(&mut pass);
        }
        let buffer=device.create_buffer(&wgpu::BufferDescriptor{label:None,size:256*256*4,usage:wgpu::BufferUsages::MAP_READ|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        encoder.copy_texture_to_buffer(color.as_image_copy(),wgpu::TexelCopyBufferInfo{buffer:&buffer,layout:wgpu::TexelCopyBufferLayout{offset:0,bytes_per_row:Some(1024),rows_per_image:Some(256)}},wgpu::Extent3d{width:256,height:256,depth_or_array_layers:1});
        queue.submit([encoder.finish()]);
        let (tx,rx)=std::sync::mpsc::channel();buffer.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());
        device.poll(wgpu::PollType::Wait).unwrap();rx.recv().unwrap().unwrap();
        let bytes=buffer.slice(..).get_mapped_range().to_vec();buffer.unmap();bytes
    };
    let baseline=draw(&mut renderer,&frame);
    assert!(baseline.chunks_exact(4).filter(|p|p[0]>10||p[1]>10||p[2]>10).count()>1000);
    let reversed:Vec<u8>=frame.mesh.chunks_exact(120).rev().flatten().copied().collect();
    frame.mesh=Arc::new(reversed);
    let reverse_pixels=draw(&mut renderer,&frame);
    assert!(baseline.chunks_exact(4).zip(reverse_pixels.chunks_exact(4)).filter(|(a,b)|a!=b).count()<30,"depth depends on submission order");
    let uploads=renderer.mesh_uploads;
    let id=circlesrust::scene::selection::pick(&scene,camera,0.5,0.5,1.0).unwrap().object_id;
    frame.camera=uniform(camera,Some(id));let selected=draw(&mut renderer,&frame);assert_ne!(baseline,selected);
    let t=ObjectTransform{translation:Vec3::new(1.0,0.0,1.0),..Default::default()};
    frame.transforms=mesh_view::transform_bytes(&scene,Some((id,t)));
    let moved=draw(&mut renderer,&frame);assert_ne!(selected,moved);
    let mut orbit=camera;circlesrust::navigation::orbit(&mut orbit,80.0,10.0);frame.camera=uniform(orbit,Some(id));
    assert_ne!(moved,draw(&mut renderer,&frame));assert_eq!(renderer.mesh_uploads,uploads);
    assert_eq!(scene.object_transforms[id],ObjectTransform::default());
    assert!(pollster::block_on(device.pop_error_scope()).is_none());
    if let Some(path)=std::env::var_os("RV_VIEWPORT_CAPTURE"){image::save_buffer(path,&selected,256,256,image::ColorType::Rgba8).unwrap();}
}
