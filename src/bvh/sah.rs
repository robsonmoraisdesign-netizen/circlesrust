// SPDX-FileCopyrightText: 2009-2011 Intel Corporation
// SPDX-FileCopyrightText: 2012-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
//! Binned surface-area heuristic for static triangles (CPU).
//! Based on clonecircles/src/bvh/binning.cpp and src/bvh/binning.h.
//! This is a compact Rust implementation, NOT the complete Cycles BVH build.
use super::Aabb;
use crate::kernel::geometry::Triangle;

const BIN_COUNT:usize=16;

#[derive(Clone,Copy)]
struct Bin {
    count:usize,
    bounds:Aabb,
}
impl Bin {
    const EMPTY:Self=Self{count:0,bounds:Aabb::empty()};
}

fn half_area(b:Aabb)->f32 {
    if b.min.x>b.max.x {return 0.0;}
    let extent=b.max-b.min;
    extent.x*extent.y+extent.y*extent.z+extent.z*extent.x
}
fn bin_index(centroid:f32,min:f32,extent:f32)->usize {
    let normalized=((centroid-min)/extent).clamp(0.0,1.0);
    ((normalized*BIN_COUNT as f32) as usize).min(BIN_COUNT-1)
}
/// Find a good spatial partition using 3-axis, 16-bin SAH sweeps.
/// Returns left primitive count after partition, or None when binning
/// cannot separate the primitives. In this case the caller uses median.
pub(super) fn partition(
    triangles:&[Triangle],order:&mut [usize],centroids:Aabb,
)->Option<usize>{
    if order.len()<2{return None;}
    let mut best_cost=f32::INFINITY;
    let mut best_axis=0usize;
    let mut best_threshold=0usize;
    for axis in 0..3 {
        let minimum=centroids.min.axis(axis);
        let extent=centroids.max.axis(axis)-minimum;
        if !extent.is_finite() || extent <= 1e-8 {continue;}
        let mut bins=[Bin::EMPTY;BIN_COUNT];
        for &primitive in order.iter(){
            let triangle=&triangles[primitive];
            let bin=bin_index(triangle.centroid().axis(axis),minimum,extent);
            bins[bin].count+=1;
            bins[bin].bounds.union(Aabb::for_triangle(triangle));
        }
        let mut right_bounds=[Aabb::empty();BIN_COUNT];
        let mut right_count=[0usize;BIN_COUNT];
        let mut box_right=Aabb::empty();
        let mut num_right=0;
        for i in (1..BIN_COUNT).rev() {
            num_right+=bins[i].count;
            box_right.union(bins[i].bounds);
            right_bounds[i]=box_right;
            right_count[i]=num_right;
        }
        let mut box_left=Aabb::empty();
        let mut num_left=0;
        for split in 1..BIN_COUNT {
            let b=bins[split-1];
            num_left+=b.count;
            box_left.union(b.bounds);
            let nr=right_count[split];
            if num_left==0 || nr==0 {continue;}
            let cost=half_area(box_left)*num_left as f32
                    +half_area(right_bounds[split])*nr as f32;
            if cost.is_finite() && cost<best_cost {
                best_cost=cost;
                best_axis=axis;
                best_threshold=split;
            }
        }
    }
    if !best_cost.is_finite(){return None;}
    let low=centroids.min.axis(best_axis);
    let extent=centroids.max.axis(best_axis)-low;
    let mut left=0usize;
    for right in 0..order.len(){
        let p=order[right];
        let bin=bin_index(triangles[p].centroid().axis(best_axis),low,extent);
        if bin<best_threshold {
            order.swap(left,right);
            left+=1;
        }
    }
    if left==0 || left==order.len(){None}else{Some(left)}
}
