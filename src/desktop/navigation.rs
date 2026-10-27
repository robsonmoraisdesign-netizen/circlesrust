// SPDX-License-Identifier: Apache-2.0
//! Blender-like camera navigation shared by Rust viewport and tests.
//! Orbit revolves about a target, pan translates origin and target together.
//! Neither navigation operation mutates mesh/scene geometry.
use crate::integrator::CameraSettings;
use crate::util::math::Vec3;

pub fn orbit(camera:&mut CameraSettings,dx:f32,dy:f32)->bool{
    if !dx.is_finite() || !dy.is_finite(){return false;}
    let d=camera.origin-camera.target;
    let radius=d.length_squared().sqrt();
    if !radius.is_finite() || radius<1e-5 {return false;}
    let yaw=d.y.atan2(d.x)-dx*0.008;
    let pitch=((d.z/radius).clamp(-1.0,1.0).asin()+dy*0.008)
        .clamp(-1.53,1.53);
    camera.origin=camera.target+Vec3::new(
        radius*pitch.cos()*yaw.cos(),
        radius*pitch.cos()*yaw.sin(),
        radius*pitch.sin()
    );
    true
}
/// Screen-space pan with perspective scaling. Dragging 100 logical pixels
/// displaces geometry approximately 100 pixels in that direction at target
/// depth, independently of the camera zoom and viewport height.
pub fn pan(
    camera:&mut CameraSettings,dx:f32,dy:f32,viewport_height:f32
)->bool{
    if !dx.is_finite()||!dy.is_finite()||
        !viewport_height.is_finite()||viewport_height<1.0{return false;}
    let forward=match (camera.target-camera.origin).normalized(){
        Some(x)=>x,None=>return false,
    };
    let hint=if forward.z.abs()>0.99{
        Vec3::new(0.0,1.0,0.0)
    }else{Vec3::new(0.0,0.0,1.0)};
    let right=match forward.cross(hint).normalized(){
        Some(x)=>x,None=>return false,
    };
    let up=right.cross(forward);
    let distance=(camera.target-camera.origin).length_squared().sqrt();
    if !distance.is_finite()||!camera.fov_degrees.is_finite(){return false;}
    let world_per_pixel=2.0*distance*
        (camera.fov_degrees.to_radians()*0.5).tan()/viewport_height;
    if !world_per_pixel.is_finite(){return false;}
    let movement=(up*dy-right*dx)*world_per_pixel;
    camera.origin=camera.origin+movement;
    camera.target=camera.target+movement;
    true
}
pub fn zoom(camera:&mut CameraSettings,scroll:f32)->bool{
    if !scroll.is_finite()||scroll==0.0 {return false;}
    let distance=camera.origin-camera.target;
    let radius=distance.length_squared().sqrt();
    if !radius.is_finite()||radius<1e-5{return false;}
    let multiplier=(-scroll*0.0015).exp();
    let clamped=(radius*multiplier).clamp(0.2,120.0);
    if let Some(normal)=distance.normalized(){
        camera.origin=camera.target+normal*clamped;
        return true;
    }
    false
}
