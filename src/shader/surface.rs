// SPDX-License-Identifier: Apache-2.0
//! Non-color maps used by an initial Rust PBR surface closure.
//! These are not the full Blender Cycles Principled BSDF / SVM closures.
use crate::shader::{ImageTexture,Uv};
use crate::util::math::Vec3;
use crate::kernel::geometry::Triangle;
use crate::integrator::Material;

#[derive(Clone,Debug,Default)]
pub struct SurfaceMaps {
    /// Linear non-color texture channel R multiplies material roughness.
    pub roughness: Option<ImageTexture>,
    /// Linear non-color texture channel R multiplies material metallic.
    pub metallic: Option<ImageTexture>,
    /// Tangent-space normal XYZ in 0..1, OpenGL (+Y) convention.
    pub normal: Option<ImageTexture>,
}
impl SurfaceMaps {
    pub fn resolved_material(&self,mut mat:Material,uv:Uv)->Material{
        if let Some(map)=&self.roughness{
            mat.roughness=(mat.roughness*map.sample(uv).x).clamp(0.045,1.0);
        }
        if let Some(map)=&self.metallic{
            mat.metallic=(mat.metallic*map.sample(uv).x).clamp(0.0,1.0);
        }
        mat
    }
    /// Tangent basis from triangle vertices/UVs and shading normal.
    /// Degenerate UVs return None; the geometric normal remains unchanged.
    pub fn mapped_normal(
        &self,triangle:&Triangle,corners:[Uv;3],uv:Uv,normal:Vec3
    )->Option<Vec3>{
        let tex=self.normal.as_ref()?;
        let delta1=triangle.b-triangle.a;
        let delta2=triangle.c-triangle.a;
        let du1=corners[1].u-corners[0].u;
        let dv1=corners[1].v-corners[0].v;
        let du2=corners[2].u-corners[0].u;
        let dv2=corners[2].v-corners[0].v;
        let det=du1*dv2-du2*dv1;
        if !det.is_finite()||det.abs()<1e-10{return None;}
        let inv=1.0/det;
        let tangent_raw=(delta1*dv2-delta2*dv1)*inv;
        let bitangent_raw=(delta2*du1-delta1*du2)*inv;
        let tangent=(tangent_raw-normal*normal.dot(tangent_raw)).normalized()?;
        let handedness=if normal.cross(tangent).dot(bitangent_raw)<0.0{-1.0}else{1.0};
        let bitangent=normal.cross(tangent)*handedness;
        let rgb=tex.sample(uv);
        let local=Vec3::new(rgb.x*2.0-1.0,rgb.y*2.0-1.0,rgb.z*2.0-1.0).normalized()?;
        let result=(tangent*local.x+bitangent*local.y+normal*local.z).normalized()?;
        // Do not perturb the normal to point beneath the original front face.
        (result.dot(triangle.normal())>0.0).then_some(result)
    }
}
