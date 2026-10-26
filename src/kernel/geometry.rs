// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
//! Triangle intersections ported from clonecircles/src/util/math_intersect.h
//! and clonecircles/src/kernel/geom/triangle_intersect.h.
use crate::util::math::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub t_min: f32,
    pub t_max: f32,
}
impl Ray {
    /// t is geometric distance only if direction is normalized.
    pub fn new(origin: Vec3, direction: Vec3, t_min: f32, t_max: f32) -> Option<Self> {
        if !origin.is_finite() || !direction.is_finite() || direction.length_squared()==0.0
           || !t_min.is_finite() || t_min<0.0 || t_max.is_nan() || t_max<=t_min {return None;}
        Some(Self{origin,direction,t_min,t_max})
    }
    pub fn at(&self, t:f32)->Vec3{self.origin+self.direction*t}
}

#[derive(Clone, Copy, Debug)]
pub struct Triangle {pub a:Vec3,pub b:Vec3,pub c:Vec3}
impl Triangle {
    pub fn new(a:Vec3,b:Vec3,c:Vec3)->Option<Self>{
        let tri=Self{a,b,c};tri.is_valid().then_some(tri)
    }
    pub fn is_valid(&self)->bool {
        self.a.is_finite()&&self.b.is_finite()&&self.c.is_finite()
         &&(self.b-self.a).cross(self.c-self.a).length_squared()>0.0
    }
    pub fn normal(&self)->Vec3 {
        (self.b-self.a).cross(self.c-self.a).normalized().unwrap_or(Vec3::ZERO)
    }
    pub fn centroid(&self)->Vec3 {
        self.a+(self.b-self.a)*(1.0/3.0)+(self.c-self.a)*(1.0/3.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Intersection {
    pub triangle_index:usize,
    pub t:f32,
    /// Coordinates of b and c; a has weight (1-u-v).
    pub u:f32,
    pub v:f32,
}
impl Intersection {pub fn point(self,ray:&Ray)->Vec3{ray.at(self.t)}}

/// Double-sided Pluecker/Embree-style edge tests from Cycles.
/// Invalid triangles must be rejected by the caller.
pub fn triangle_intersect(ray:&Ray, tri:&Triangle, max_t:f32)->Option<(f32,f32,f32)> {
    let v0=tri.a-ray.origin;
    let v1=tri.b-ray.origin;
    let v2=tri.c-ray.origin;
    let e0=v2-v0;
    let e1=v0-v1;
    let e2=v1-v2;
    let u=e0.cross(v2+v0).dot(ray.direction);
    let v=e1.cross(v0+v1).dot(ray.direction);
    let w=e2.cross(v1+v2).dot(ray.direction);
    let uvw=u+v+w;
    let eps=f32::EPSILON*uvw.abs();
    if !(u.min(v.min(w))>=-eps || u.max(v.max(w))<=eps){return None;}
    let n=e1.cross(e0)*2.0;
    let den=n.dot(ray.direction);
    if den==0.0 || !den.is_finite(){return None;}
    let t=v0.dot(n)/den;
    if !t.is_finite() || !(t>=ray.t_min && t<=ray.t_max.min(max_t)){return None;}
    let rcp=if uvw.abs()<1e-18{0.0}else{1.0/uvw};
    Some((t,(u*rcp).min(1.0),(v*rcp).min(1.0)))
}
