// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
//! Procedural test scene to validate the Rust path integrator.
//! The Cycles scene/object/mesh node graph has not been ported yet.

use crate::integrator::{Material, RectLight, Scene};
use crate::kernel::geometry::Triangle;
use crate::util::math::Vec3;

fn v(x:f32,y:f32,z:f32)->Vec3 {Vec3::new(x,y,z)}
struct MeshBuilder {tris:Vec<Triangle>, materials:Vec<usize>}
impl MeshBuilder {
    fn new()->Self {Self{tris:Vec::new(),materials:Vec::new()}}
    fn triangle(&mut self,a:Vec3,b:Vec3,c:Vec3,material:usize){
        if let Some(t)=Triangle::new(a,b,c) {
            self.tris.push(t);self.materials.push(material);
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
                self.triangle(a,b,c,material);
                self.triangle(a,c,d,material);
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
    Scene::new(mesh.tris,mesh.materials,materials,light)
        .expect("studio scene mesh must be valid")
}
