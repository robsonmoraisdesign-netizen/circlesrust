// SPDX-License-Identifier: Apache-2.0
//! World-axis transform gestures from an egui viewport's pointer delta.
//! This module is native Rust, independent of widgets and HTML.
use crate::integrator::CameraSettings;
use crate::scene::transform::ObjectTransform;
use crate::util::math::Vec3;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EditTool{Select,Move,Rotate,Scale}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EditAxis{Free,X,Y,Z}
fn constrained(v:Vec3,axis:EditAxis)->Vec3{
    match axis{
        EditAxis::Free=>v,
        EditAxis::X=>Vec3::new(v.x,0.0,0.0),
        EditAxis::Y=>Vec3::new(0.0,v.y,0.0),
        EditAxis::Z=>Vec3::new(0.0,0.0,v.z)
    }
}
/// Drag in camera plane for movement. Rotation uses world axes, scale is positive.
pub fn apply_drag(
    t:ObjectTransform,tool:EditTool,axis:EditAxis,dx:f32,dy:f32,
    camera:CameraSettings,viewport_height:f32
)->Option<ObjectTransform>{
    if !dx.is_finite()||!dy.is_finite()||!viewport_height.is_finite()||
        viewport_height<1.0||camera.validate().is_err(){return None;}
    let mut result=t;
    match tool{
        EditTool::Select=>return None,
        EditTool::Move=>{
            let forward=(camera.target-camera.origin).normalized()?;
            let up_hint=if forward.z.abs()>0.99{
                Vec3::new(0.0,1.0,0.0)
            }else{Vec3::new(0.0,0.0,1.0)};
            let right=forward.cross(up_hint).normalized()?;
            let up=right.cross(forward);
            let distance=(camera.target-camera.origin).length_squared().sqrt();
            let scale=2.0*distance*(camera.fov_degrees.to_radians()*0.5).tan()/viewport_height;
            result.translation=result.translation+
                constrained((right*dx-up*dy)*scale,axis);
        }
        EditTool::Rotate=>{
            let degrees=dx*0.42-dy*0.12;
            let amount=match axis {
                EditAxis::X=>Vec3::new(degrees,0.0,0.0),
                EditAxis::Y=>Vec3::new(0.0,degrees,0.0),
                _=>Vec3::new(0.0,0.0,degrees)
            };
            result.rotation_degrees=result.rotation_degrees+amount;
        }
        EditTool::Scale=>{
            let factor=(-dy*0.012).exp().clamp(0.1,10.0);
            let old=result.scale;
            result.scale=match axis{
                EditAxis::Free=>old*factor,
                EditAxis::X=>Vec3::new(old.x*factor,old.y,old.z),
                EditAxis::Y=>Vec3::new(old.x,old.y*factor,old.z),
                EditAxis::Z=>Vec3::new(old.x,old.y,old.z*factor),
            };
        }
    }
    result.valid().then_some(result)
}
