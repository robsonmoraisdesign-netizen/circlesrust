// SPDX-License-Identifier: Apache-2.0 AND MIT
//! Column-major matrix operations adapted from SolidCraft render/camera.rs.
//! See ATTRIBUTION.md and third_party/SOLIDCRAFT-MIT.txt.
use crate::{integrator::{CameraSettings, Scene}, scene::transform::ObjectTransform, util::math::Vec3};
#[derive(Clone, Copy, Debug)]
pub struct Mat4(pub [[f32; 4]; 4]);
impl Mat4 {
    pub fn apply(self, p: Vec3) -> [f32;4] {
        let v=[p.x,p.y,p.z,1.0];
        std::array::from_fn(|r| (0..4).map(|c|self.0[c][r]*v[c]).sum())
    }
    pub fn bytes(self) -> Vec<u8> {
        self.0.iter().flatten().flat_map(|v|v.to_le_bytes()).collect()
    }
}
pub fn view_projection(camera: CameraSettings, aspect:f32)->Option<Mat4>{
    let camera=camera.validate().ok()?;
    if !aspect.is_finite() || aspect<=0.0 {return None;}
    let f=(camera.target-camera.origin).normalized()?;
    let hint=if f.z.abs()>0.99 {Vec3::new(0.0,1.0,0.0)} else {Vec3::new(0.0,0.0,1.0)};
    let r=f.cross(hint).normalized()?;
    let u=r.cross(f);
    let e=camera.origin;
    let view=Mat4([[r.x,u.x,-f.x,0.0],[r.y,u.y,-f.y,0.0],[r.z,u.z,-f.z,0.0],[-r.dot(e),-u.dot(e),f.dot(e),1.0]]);
    let t=1.0/(camera.fov_degrees.to_radians()*0.5).tan();
    let (n,z)=(0.001,1000.0);
    let proj=Mat4([[t/aspect,0.0,0.0,0.0],[0.0,t,0.0,0.0],[0.0,0.0,z/(n-z),-1.0],[0.0,0.0,n*z/(n-z),0.0]]);
    Some(Mat4(std::array::from_fn(|c| std::array::from_fn(|r| (0..4).map(|k|proj.0[k][r]*view.0[c][k]).sum()))))
}
pub fn object_matrices(t:ObjectTransform,pivot:Vec3)->(Mat4,Mat4){
    let basis=[Vec3::new(1.0,0.0,0.0),Vec3::new(0.0,1.0,0.0),Vec3::new(0.0,0.0,1.0)];
    let scales=[t.scale.x,t.scale.y,t.scale.z];
    let mut m=[[0.0;4];4];let mut n=m;
    for i in 0..3 {
        let a=t.rotate(basis[i])*scales[i];
        let b=t.rotate(basis[i])/scales[i];
        m[i]=[a.x,a.y,a.z,0.0];n[i]=[b.x,b.y,b.z,0.0];
    }
    let a=t.point(Vec3::ZERO,pivot);
    m[3]=[a.x,a.y,a.z,1.0];n[3][3]=1.0;
    (Mat4(m),Mat4(n))
}
pub fn mesh_bytes(scene:&Scene)->Vec<u8>{
    let mut out=Vec::with_capacity(scene.base_triangles.len()*120);
    for (i,t) in scene.base_triangles.iter().enumerate(){
        let flat=(t.b-t.a).cross(t.c-t.a).normalized().unwrap_or(Vec3::ZERO);
        let normals=scene.base_corner_normals[i].unwrap_or([flat;3]);
        let color=scene.materials[scene.triangle_materials[i]].base_color;
        for (p,n) in [t.a,t.b,t.c].into_iter().zip(normals){
            for v in [p.x,p.y,p.z,n.x,n.y,n.z,color.x,color.y,color.z]{out.extend(v.to_le_bytes());}
            out.extend((scene.triangle_objects[i] as u32).to_le_bytes());
        }
    }
    out
}
pub fn transform_bytes(scene:&Scene,preview:Option<(usize,ObjectTransform)>)->Vec<u8>{
    let mut out=Vec::with_capacity(scene.object_transforms.len()*128);
    for (id,&committed) in scene.object_transforms.iter().enumerate(){
        let t=preview.filter(|(i,_)|*i==id).map(|(_,t)|t).unwrap_or(committed);
        let (m,n)=object_matrices(t,scene.object_pivots[id]);out.extend(m.bytes());out.extend(n.bytes());
    }
    out
}
pub fn camera_bytes(camera:CameraSettings,aspect:f32,light:Vec3,selection:Option<usize>,tint:Vec3,srgb:bool)->Option<Vec<u8>>{
    let mut out=view_projection(camera,aspect)?.bytes();
    for v in [light.x,light.y,light.z,0.0,camera.origin.x,camera.origin.y,camera.origin.z,if srgb{1.0}else{0.0},tint.x,tint.y,tint.z,selection.map(|i|i as f32).unwrap_or(-1.0)]{out.extend(v.to_le_bytes());}
    Some(out)
}
