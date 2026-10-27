// SPDX-License-Identifier: Apache-2.0
//! Actual object picking from the Rust BVH and camera framing.
//! Source identifiers correspond to OBJ object/group names, not materials.
use crate::bvh::Aabb;
use crate::integrator::{CameraSettings,Scene};
use crate::kernel::geometry::Ray;
use crate::util::math::Vec3;

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct PickedObject{
    pub object_id:usize,
    pub triangle_id:usize,
    pub material_id:usize,
    pub hit_position:Vec3,
}

pub fn pick(
    scene:&Scene,camera:CameraSettings,nx:f32,ny:f32,aspect:f32
)->Option<PickedObject>{
    let camera=camera.validate().ok()?;
    if !nx.is_finite()||!ny.is_finite()||!aspect.is_finite()||aspect<=0.0
        ||!(0.0..=1.0).contains(&nx)||!(0.0..=1.0).contains(&ny){return None;}
    let forward=(camera.target-camera.origin).normalized()?;
    let up_hint=if forward.z.abs()>0.99{
        Vec3::new(0.0,1.0,0.0)
    }else{Vec3::new(0.0,0.0,1.0)};
    let right=forward.cross(up_hint).normalized()?;
    let up=right.cross(forward);
    let tangent=(camera.fov_degrees.to_radians()*0.5).tan();
    let sx=(2.0*nx-1.0)*aspect*tangent;
    let sy=(1.0-2.0*ny)*tangent;
    let direction=(forward+right*sx+up*sy).normalized()?;
    let ray=Ray::new(camera.origin,direction,0.001,1000.0)?;
    let hit=scene.bvh.closest_hit(&ray)?;
    Some(PickedObject{
        object_id:scene.triangle_objects[hit.triangle_index],
        material_id:scene.triangle_materials[hit.triangle_index],
        triangle_id:hit.triangle_index,
        hit_position:hit.point(&ray),
    })
}
pub fn object_bounds(scene:&Scene,id:usize)->Option<Aabb>{
    if id>=scene.object_names.len(){return None;}
    let mut bounds=Aabb::empty();
    let mut found=false;
    for (i,&group) in scene.triangle_objects.iter().enumerate(){
        if group==id {
            if let Some(triangle)=scene.bvh.triangle(i){
                bounds.union(Aabb::for_triangle(triangle));
                found=true;
            }
        }
    }
    found.then_some(bounds)
}
pub fn frame_object(
    scene:&Scene,camera:&mut CameraSettings,id:usize,aspect:f32
)->bool {
    if camera.validate().is_err()||!aspect.is_finite()||aspect<=0.0{return false;}
    let Some(bounds)=object_bounds(scene,id) else{return false;};
    let Some(direction)=(camera.target-camera.origin).normalized() else{return false;};
    let center=bounds.center();
    let half=bounds.max-center;
    let radius=half.length_squared().sqrt().max(0.05);
    let tangent=(camera.fov_degrees.to_radians()*0.5).tan();
    let distance=(radius*1.45/(tangent*aspect.min(1.0))).max(radius+0.35);
    let origin=center-direction*distance;
    if !origin.is_finite(){return false;}
    camera.origin=origin;
    camera.target=center;
    true
}
