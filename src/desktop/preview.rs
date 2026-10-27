// SPDX-License-Identifier: Apache-2.0
//! Instant solid/Workbench-style scene visibility before any path-traced render.
//! CPU raycast into the real BVH. No Monte Carlo samples, denoiser or fake image.
//! The GUI may request a smaller preview while navigating, independently of
//! the final output resolution.
use std::thread;
use crate::integrator::{CameraSettings,Scene};
use crate::kernel::geometry::Ray;
use crate::util::math::Vec3;
use crate::shader::Uv;

fn rgb(x:Vec3)->[u8;3]{
    let to_byte=|v:f32| ((v.clamp(0.0,1.0).powf(1.0/2.2))*255.0+0.5) as u8;
    [to_byte(x.x),to_byte(x.y),to_byte(x.z)]
}
fn mul(a:Vec3,b:Vec3)->Vec3{Vec3::new(a.x*b.x,a.y*b.y,a.z*b.z)}
/// Render a shaded solid image from actual meshes, with z-occlusion via BVH.
/// Same camera projection as the CPU path tracer, including orbit adjustments.
pub fn solid_preview(
    scene:&Scene,camera:CameraSettings,width:u32,height:u32,
)->Result<Vec<u8>,&'static str>{
    let camera=camera.validate()?;
    if !(1..=2048).contains(&width)||!(1..=2048).contains(&height){
        return Err("Solid preview dimensions must be 1..=2048");
    }
    let forward=(camera.target-camera.origin).normalized().ok_or("Invalid camera")?;
    let up_hint=if forward.z.abs()>0.99{
        Vec3::new(0.0,1.0,0.0)
    }else{
        Vec3::new(0.0,0.0,1.0)
    };
    let right=forward.cross(up_hint).normalized().ok_or("Invalid camera basis")?;
    let up=right.cross(forward);
    let tangent=(camera.fov_degrees.to_radians()*0.5).tan();
    let aspect=width as f32/height as f32;
    let w=width as usize;
    let h=height as usize;
    let mut pixels=vec![0u8;w*h*3];
    let worker_count=thread::available_parallelism().map(|p|p.get()).unwrap_or(1).min(h);
    let chunk_rows=h.div_ceil(worker_count);
    thread::scope(|scope|{
        for (block,part) in pixels.chunks_mut(chunk_rows*w*3).enumerate(){
            scope.spawn(move||{
                let start_y=block*chunk_rows;
                for (local_y,line) in part.chunks_exact_mut(w*3).enumerate(){
                    let y=start_y+local_y;
                    for (x,px) in line.as_chunks_mut::<3>().0.iter_mut().enumerate(){
                        let sx=(2.0*(x as f32+0.5)/width as f32-1.0)*aspect*tangent;
                        let sy=(1.0-2.0*(y as f32+0.5)/height as f32)*tangent;
                        let Some(direction)=(forward+right*sx+up*sy).normalized() else{continue;};
                        let Some(ray)=Ray::new(camera.origin,direction,0.001,1000.0) else{continue;};
                        let out=if let Some(hit)=scene.bvh.closest_hit(&ray){
                            let index=scene.triangle_materials[hit.triangle_index];
                            let material=scene.materials[index];
                            let mut base=material.base_color;
                            if let (Some(corners),Some(graph))=(
                                scene.corner_uvs[hit.triangle_index],scene.shader_graphs[index].as_ref()
                            ){
                                base=graph.evaluate(Uv::interpolate(corners,hit.u,hit.v));
                            }
                            let mut normal=scene.mapped_shading_normal(
                                hit.triangle_index,hit.u,hit.v
                            );
                            if normal.dot(direction)>0.0{normal=-normal;}
                            let to_light=(scene.light.center-hit.point(&ray))
                                .normalized().unwrap_or(Vec3::new(0.0,0.0,1.0));
                            let lambert=normal.dot(to_light).max(0.0);
                            let ambient=Vec3::new(0.26,0.29,0.35);
                            let lighting=ambient+Vec3::new(0.9,0.88,0.84)*lambert*0.75;
                            let metal_tint=Vec3::new(0.94,0.96,1.0);
                            let lit=mul(base,lighting);
                            let gloss=normal.dot((to_light-direction)
                                .normalized().unwrap_or(to_light)).max(0.0);
                            lit+mul(base,metal_tint)*gloss.powf(20.0)*(0.08+0.14*material.metallic)
                        }else{
                            let t=(direction.z*0.5+0.5).clamp(0.0,1.0);
                            Vec3::new(0.055,0.075,0.11)*(1.0-t)
                                +Vec3::new(0.17,0.22,0.29)*t
                        };
                        *px=rgb(out);
                    }
                }
            });
        }
    });
    Ok(pixels)
}
