// SPDX-License-Identifier: Apache-2.0
//! Per-object native Rust geometry transforms. BVH2 is rebuilt after edits.
use crate::bvh::Bvh;
use crate::integrator::Scene;
use crate::kernel::geometry::Triangle;
use crate::util::math::Vec3;

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct ObjectTransform{
    pub translation:Vec3,
    /// Euler XYZ rotation in degrees.
    pub rotation_degrees:Vec3,
    /// Positive scale on each axis. Mirroring is not implemented.
    pub scale:Vec3,
}
impl Default for ObjectTransform{
    fn default()->Self{
        Self{translation:Vec3::ZERO,rotation_degrees:Vec3::ZERO,
            scale:Vec3::new(1.0,1.0,1.0)}
    }
}
impl ObjectTransform{
    pub fn valid(self)->bool{
        let t=self.translation;
        let r=self.rotation_degrees;
        let s=self.scale;
        t.is_finite()&&r.is_finite()&&s.is_finite()&&
        [t.x,t.y,t.z].iter().all(|x|x.abs()<=100_000.0)&&
        [r.x,r.y,r.z].iter().all(|x|x.abs()<=3600.0)&&
        [s.x,s.y,s.z].iter().all(|x|(0.01..=100.0).contains(x))
    }
    fn rotate(self,v:Vec3)->Vec3{
        let (sx,cx)=self.rotation_degrees.x.to_radians().sin_cos();
        let (sy,cy)=self.rotation_degrees.y.to_radians().sin_cos();
        let (sz,cz)=self.rotation_degrees.z.to_radians().sin_cos();
        let a=Vec3::new(v.x,v.y*cx-v.z*sx,v.y*sx+v.z*cx);
        let b=Vec3::new(a.x*cy+a.z*sy,a.y,-a.x*sy+a.z*cy);
        Vec3::new(b.x*cz-b.y*sz,b.x*sz+b.y*cz,b.z)
    }
    fn point(self,p:Vec3,pivot:Vec3)->Vec3{
        let d=p-pivot;
        let scaled=Vec3::new(d.x*self.scale.x,d.y*self.scale.y,d.z*self.scale.z);
        pivot+self.translation+self.rotate(scaled)
    }
    fn normal(self,n:Vec3)->Vec3{
        self.rotate(Vec3::new(
            n.x/self.scale.x,n.y/self.scale.y,n.z/self.scale.z
        )).normalized().unwrap_or(Vec3::ZERO)
    }
}
impl Scene{
    /// Rebuild from original geometry, avoiding cumulative numerical drift.
    /// Preserve material slots, UV coordinates, and unrelated object positions.
    pub fn apply_object_transform(
        &mut self,id:usize,value:ObjectTransform
    )->Result<(),&'static str>{
        if id>=self.object_names.len(){return Err("Object index out of range");}
        if !value.valid(){return Err("Invalid object transform values");}
        if self.object_transforms[id]==value{return Ok(());}
        let mut transforms=self.object_transforms.clone();
        transforms[id]=value;
        let mut triangles=Vec::with_capacity(self.base_triangles.len());
        let mut normals=Vec::with_capacity(self.base_corner_normals.len());
        for (i,base) in self.base_triangles.iter().enumerate(){
            let object=self.triangle_objects[i];
            let transform=transforms[object];
            let pivot=self.object_pivots[object];
            if transform==ObjectTransform::default(){
                // Preserve exact source bits for untouched objects and reset.
                // Re-centering with f32 would otherwise perturb tiny pole faces.
                triangles.push(*base);
                normals.push(self.base_corner_normals[i]);
            }else{
                let a=transform.point(base.a,pivot);
                let b=transform.point(base.b,pivot);
                let c=transform.point(base.c,pivot);
                if !a.is_finite()||!b.is_finite()||!c.is_finite(){
                    return Err("Nonfinite transformed geometry");
                }
                // Extremely small triangles may collapse in f32 under translation.
                // Keep their indices for UV/material mapping; BVH skips zero-area
                // faces instead of aborting the entire object edit.
                triangles.push(Triangle{a,b,c});
                normals.push(self.base_corner_normals[i].map(|n|
                    n.map(|value|transform.normal(value))
                ));
            }
        }
        self.bvh=Bvh::build(triangles);
        self.corner_normals=normals;
        self.object_transforms=transforms;
        Ok(())
    }
}
