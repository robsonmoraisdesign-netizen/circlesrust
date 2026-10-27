pub mod selection;
pub mod loader;
pub mod mtl;
// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
// Procedural test scene to validate the Rust path integrator.
// The Cycles scene/object/mesh node graph has not been ported yet.
pub mod obj;

use crate::integrator::{Material, RectLight, Scene};
use crate::kernel::geometry::Triangle;
use crate::util::math::Vec3;
use crate::shader::Uv;

fn v(x:f32,y:f32,z:f32)->Vec3 {Vec3::new(x,y,z)}
struct MeshBuilder {
    tris:Vec<Triangle>,
    materials:Vec<usize>,
    normals:Vec<Option<[Vec3;3]>>,
    uvs:Vec<Option<[Uv;3]>>,
}
impl MeshBuilder {
    fn new()->Self {Self{tris:Vec::new(),materials:Vec::new(),normals:Vec::new(),uvs:Vec::new()}}
    fn triangle(&mut self,a:Vec3,b:Vec3,c:Vec3,material:usize){
        if let Some(t)=Triangle::new(a,b,c) {
            self.tris.push(t);self.materials.push(material);self.normals.push(None);self.uvs.push(None);
        }
    }
    fn smooth_triangle(&mut self, vertices:[Vec3;3], corner_normals:[Vec3;3], material:usize) {
        if let Some(triangle) = Triangle::new(vertices[0],vertices[1],vertices[2]) {
            self.tris.push(triangle);
            self.materials.push(material);
            self.normals.push(Some(corner_normals));self.uvs.push(None);
        }
    }
    fn quad(&mut self,a:Vec3,b:Vec3,c:Vec3,d:Vec3,material:usize){
        self.triangle(a,b,c,material);
        self.triangle(a,c,d,material);
    }
    fn cube(&mut self,center:Vec3,half:f32,material:usize){
        let vertices=[
            v(-1.0,-1.0,-1.0),v(1.0,-1.0,-1.0),
            v(1.0,1.0,-1.0),v(-1.0,1.0,-1.0),
            v(-1.0,-1.0,1.0),v(1.0,-1.0,1.0),
            v(1.0,1.0,1.0),v(-1.0,1.0,1.0),
        ].map(|p|center+p*half);
        for [a,b,c,d] in [
            [0,3,2,1], [4,5,6,7], [0,1,5,4],
            [1,2,6,5], [2,3,7,6], [3,0,4,7],
        ] {
            self.quad(vertices[a],vertices[b],vertices[c],vertices[d],material);
        }
    }
    fn sphere(&mut self,center:Vec3,radius:f32,material:usize){
        const RINGS:u32=24;
        const SEGMENTS:u32=40;
        let point=|ring:u32,segment:u32|{
            let theta=std::f32::consts::PI*ring as f32/RINGS as f32;
            let phi=2.0*std::f32::consts::PI*segment as f32/SEGMENTS as f32;
            center+v(theta.sin()*phi.cos(),theta.sin()*phi.sin(),theta.cos())*radius
        };
        for j in 0..RINGS{
            for i in 0..SEGMENTS{
                let a=point(j,i);
                let b=point(j+1,i);
                let c=point(j+1,i+1);
                let d=point(j,i+1);
                self.smooth_triangle([a,b,c],[(a-center)/radius,(b-center)/radius,(c-center)/radius],material);
                self.smooth_triangle([a,c,d],[(a-center)/radius,(c-center)/radius,(d-center)/radius],material);
            }
        }
    }
}

/// Demonstration of direct area illumination, floor bounce, GGX materials,
/// soft shadows, antialiasing and glossy reflections. Geometry is static.
pub fn studio_scene()->Scene {
    let materials=vec![
        Material::new(v(0.57,0.62,0.70),0.0,0.78), // neutral studio ground
        Material::new(v(0.28,0.35,0.46),0.0,0.93), // wall
        Material::new(v(0.87,0.40,0.16),0.12,0.34), // amber clay-like gloss
        Material::new(v(0.33,0.52,0.72),0.75,0.22), // metallic blue
        Material::new(v(0.78,0.73,0.59),0.90,0.23), // brushed champagne metal
    ];
    let mut mesh=MeshBuilder::new();
    mesh.quad(v(-8.0,-8.0,0.0),v(8.0,-8.0,0.0),v(8.0,8.0,0.0),v(-8.0,8.0,0.0),0);
    mesh.quad(v(-8.0,4.5,0.0),v(8.0,4.5,0.0),v(8.0,4.5,8.0),v(-8.0,4.5,8.0),1);
    mesh.cube(v(-0.85,0.40,0.98),0.98,2);
    mesh.cube(v(1.45,0.75,0.52),0.52,4);
    mesh.sphere(v(0.8,-1.04,0.66),0.66,3);
    let light=RectLight{
        center:v(-0.5,-0.35,5.8),
        half_u:v(1.75,0.0,0.0),
        half_v:v(0.0,-1.25,0.0),
        radiance:v(9.0,8.2,7.4),
    };
    let objects=mesh.materials.iter().map(|m|match m{
        0=>0,1=>1,2=>2,4=>3,3=>4,_=>0
    }).collect();
    let mut scene = Scene::new(mesh.tris,mesh.materials,materials,light)
        .expect("studio scene mesh must be valid");
    scene.set_corner_normals(mesh.normals).expect("valid generated normals");
    scene.set_objects(objects,vec![
        "Piso".into(),"Fundo".into(),"Cubo cerâmico".into(),
        "Cubo metal".into(),"Esfera azul".into()
    ]).expect("valid studio object mapping");
    scene.set_corner_uvs(mesh.uvs).expect("valid generated UVs");
    scene
}


/// Render an OBJ in the same studio with an optional set of MTL materials.
/// The original obj_studio_scene entry point keeps its neutral fallback.
pub fn obj_studio_scene(mesh:obj::ObjMesh)->Scene{
    obj_studio_scene_with_materials(mesh,&[])
}
/// Apply per-face material assignments by OBJ usemtl, with sane default for
/// missing/unsupported library entries. All shading is still Rust CPU GGX.
pub fn obj_studio_scene_with_materials(
    mut mesh:obj::ObjMesh, named:&[mtl::NamedMaterial]
)->Scene{
    mesh.fit_to_studio();
    let mut geometry=MeshBuilder::new();
    geometry.quad(v(-8.0,-8.0,-0.02),v(8.0,-8.0,-0.02),
        v(8.0,8.0,-0.02),v(-8.0,8.0,-0.02),0);
    let mut materials=vec![
        Material::new(v(0.45,0.48,0.51),0.0,0.85),
        Material::new(v(0.66,0.68,0.72),0.2,0.32),
    ];
    for item in named {
        materials.push(item.material);
    }
    let group_names=mesh.group_names.clone();
    let names=std::iter::once("Piso".to_owned())
        .chain(group_names.iter().cloned()).collect::<Vec<_>>();
    let mut objects=vec![0usize;geometry.tris.len()];
    for ((((tri,normals),uvs),face_material),face_group) in mesh.triangles.into_iter()
        .zip(mesh.corner_normals)
        .zip(mesh.corner_uvs)
        .zip(mesh.face_materials)
        .zip(mesh.face_groups){
        let material_index=face_material.as_deref()
            .and_then(|name|named.iter().position(|m|m.name==name))
            .map(|index|index+2)
            .unwrap_or(1);
        geometry.tris.push(tri);
        geometry.materials.push(material_index);
        geometry.normals.push(normals);
        geometry.uvs.push(uvs);
        let group_index=group_names.iter().position(|g|g==&face_group).unwrap_or(0);
        objects.push(group_index+1);
    }
    let light=RectLight{
        center:v(-0.50,-0.25,5.7),
        half_u:v(1.75,0.0,0.0),
        half_v:v(0.0,-1.25,0.0),
        radiance:v(9.0,8.2,7.4),
    };
    let mut scene=Scene::new(geometry.tris,geometry.materials,materials,light)
        .expect("imported OBJ scene is valid");
    scene.set_corner_normals(geometry.normals).expect("OBJ generated normals are valid");
    scene.set_corner_uvs(geometry.uvs).expect("OBJ generated UVs are valid");
    scene.set_objects(objects,names).expect("OBJ object mapping is valid");
    scene
}
