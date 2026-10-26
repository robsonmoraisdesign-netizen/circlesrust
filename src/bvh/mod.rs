// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
//! Static triangle CPU BVH2 implementation, adapted from:
//! clonecircles/src/bvh/{build.cpp,bvh2.cpp,node.h,params.h}
//! clonecircles/src/util/{boundbox.h,math_intersect.h}
//! Limited to binary nodes and centroid median splitting. SAH spatial splits,
//! motion, instances, hair/curves, packed GPU layouts and refitting are TODO.

use crate::kernel::geometry::{triangle_intersect, Intersection, Ray, Triangle};
use crate::util::math::Vec3;

/// Cycles BVHParams default maximum triangle leaf size.
pub const MAX_LEAF_TRIANGLES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}
impl Aabb {
    pub const fn empty() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }
    pub fn for_triangle(t:&Triangle)->Self {
        let mut b=Self::empty();
        b.grow(t.a);b.grow(t.b);b.grow(t.c);b
    }
    pub fn grow(&mut self,p:Vec3) {
        self.min=self.min.min(p);self.max=self.max.max(p);
    }
    pub fn union(&mut self,b:Self) {
        self.min=self.min.min(b.min);self.max=self.max.max(b.max);
    }
    pub fn center(&self)->Vec3 {
        self.min+(self.max-self.min)*0.5
    }
    /// Slab test with explicit parallel-ray handling, avoiding 0*infinity NaNs.
    /// Returns the entry parameter of the intersected ray interval.
    pub fn entry_distance(&self,ray:&Ray,max_t:f32)->Option<f32>{
        if self.min.x>self.max.x || self.min.y>self.max.y || self.min.z>self.max.z{return None;}
        let mut near=ray.t_min;
        let mut far=ray.t_max.min(max_t);
        for axis in 0..3{
            let origin=ray.origin.axis(axis);
            let direction=ray.direction.axis(axis);
            let low=self.min.axis(axis);
            let high=self.max.axis(axis);
            if direction==0.0{
                if origin<low || origin>high{return None;}
                continue;
            }
            let mut a=(low-origin)/direction;
            let mut b=(high-origin)/direction;
            if a>b{std::mem::swap(&mut a,&mut b);}
            near=near.max(a);
            far=far.min(b);
            if near>far{return None;}
        }
        Some(near)
    }
}

#[derive(Clone,Copy,Debug)]
enum Kind { Leaf {start:usize,end:usize}, Branch {left:usize,right:usize} }
#[derive(Clone,Copy,Debug)]
struct Node {bounds:Aabb,kind:Kind}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct BvhStats {
    pub nodes:usize,
    pub leaves:usize,
    pub triangles:usize,
    pub maximum_depth:usize,
}

/// Binary hierarchy for independent static triangles (not a full Cycles BVH).
pub struct Bvh {
    triangles:Vec<Triangle>,
    order:Vec<usize>,
    nodes:Vec<Node>,
    maximum_depth:usize,
}
impl Bvh {
    /// Invalid triangles are skipped, while hits retain original input indices.
    pub fn build(triangles:Vec<Triangle>)->Self {
        let order=triangles.iter().enumerate()
            .filter_map(|(i,t)|t.is_valid().then_some(i)).collect();
        let mut tree=Self{triangles,order,nodes:Vec::new(),maximum_depth:0};
        if !tree.order.is_empty(){
            let len=tree.order.len();tree.build_node(0,len,1);
        }
        tree
    }
    fn build_node(&mut self,start:usize,end:usize,depth:usize)->usize {
        self.maximum_depth=self.maximum_depth.max(depth);
        let mut bounds=Aabb::empty();
        let mut centroids=Aabb::empty();
        for &i in &self.order[start..end]{
            let tri=&self.triangles[i];
            bounds.union(Aabb::for_triangle(tri));
            centroids.grow(tri.centroid());
        }
        let here=self.nodes.len();
        self.nodes.push(Node{bounds,kind:Kind::Leaf{start,end}});
        if end-start<=MAX_LEAF_TRIANGLES || depth>=64{return here;}

        let extent=centroids.max-centroids.min;
        let axis=if extent.x>=extent.y && extent.x>=extent.z{0}
            else if extent.y>=extent.z{1}else{2};
        let tris=&self.triangles;
        self.order[start..end].sort_unstable_by(|a,b|{
            tris[*a].centroid().axis(axis)
                .total_cmp(&tris[*b].centroid().axis(axis))
                .then_with(||a.cmp(b))
        });
        let mid=start+(end-start)/2;
        let left=self.build_node(start,mid,depth+1);
        let right=self.build_node(mid,end,depth+1);
        self.nodes[here].kind=Kind::Branch{left,right};
        here
    }
    pub fn triangle(&self,index:usize)->Option<&Triangle>{self.triangles.get(index)}
    pub fn stats(&self)->BvhStats {
        BvhStats{
            nodes:self.nodes.len(),
            leaves:self.nodes.iter().filter(|n|matches!(n.kind,Kind::Leaf{..})).count(),
            triangles:self.order.len(),
            maximum_depth:self.maximum_depth,
        }
    }
    /// Return the closest intersection, culling distant nodes as closer hits arrive.
    pub fn closest_hit(&self,ray:&Ray)->Option<Intersection>{
        if self.nodes.is_empty(){return None;}
        let mut best=ray.t_max;
        let mut result=None;
        let mut stack=vec![0usize];
        while let Some(index)=stack.pop(){
            let node=self.nodes[index];
            if node.bounds.entry_distance(ray,best).is_none(){continue;}
            match node.kind{
                Kind::Leaf{start,end}=>{
                    for &i in &self.order[start..end]{
                        if let Some((t,u,v))=triangle_intersect(ray,&self.triangles[i],best){
                            if t<=best{
                                best=t;
                                result=Some(Intersection{triangle_index:i,t,u,v});
                            }
                        }
                    }
                }
                Kind::Branch{left,right}=>{
                    let l=self.nodes[left].bounds.entry_distance(ray,best);
                    let r=self.nodes[right].bounds.entry_distance(ray,best);
                    match (l,r){
                        (Some(a),Some(b)) if a<=b=>{stack.push(right);stack.push(left);}
                        (Some(_),Some(_))=>{stack.push(left);stack.push(right);}
                        (Some(_),None)=>stack.push(left),
                        (None,Some(_))=>stack.push(right),
                        (None,None)=>{},
                    }
                }
            }
        }
        result
    }
    /// True if any opaque triangle lies within this ray's t-interval.
    pub fn occluded(&self,ray:&Ray)->bool{
        if self.nodes.is_empty(){return false;}
        let mut stack=vec![0usize];
        while let Some(index)=stack.pop(){
            let node=self.nodes[index];
            if node.bounds.entry_distance(ray,ray.t_max).is_none(){continue;}
            match node.kind{
                Kind::Leaf{start,end}=>{
                    for &i in &self.order[start..end]{
                        if triangle_intersect(ray,&self.triangles[i],ray.t_max).is_some(){
                            return true;
                        }
                    }
                }
                Kind::Branch{left,right}=>{stack.push(right);stack.push(left);}
            }
        }
        false
    }
}
