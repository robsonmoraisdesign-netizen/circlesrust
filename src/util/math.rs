// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
//! Vec3 math adapted from Cycles util/math_float3.h.
use std::ops::{Add, Sub, Mul, Div, Neg};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    pub fn dot(self, rhs: Self) -> f32 { self.x*rhs.x+self.y*rhs.y+self.z*rhs.z }
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(self.y*rhs.z-self.z*rhs.y, self.z*rhs.x-self.x*rhs.z, self.x*rhs.y-self.y*rhs.x)
    }
    pub fn length_squared(self) -> f32 { self.dot(self) }
    pub fn normalized(self) -> Option<Self> {
        let len_sq = self.length_squared();
        if len_sq > 0.0 && len_sq.is_finite() { Some(self / len_sq.sqrt()) } else { None }
    }
    pub fn is_finite(self) -> bool { self.x.is_finite() && self.y.is_finite() && self.z.is_finite() }
    pub fn axis(self, axis: usize) -> f32 {
        match axis { 0=>self.x, 1=>self.y, 2=>self.z, _=>panic!("invalid axis") }
    }
    pub fn min(self, b: Self) -> Self { Self::new(self.x.min(b.x),self.y.min(b.y),self.z.min(b.z)) }
    pub fn max(self, b: Self) -> Self { Self::new(self.x.max(b.x),self.y.max(b.y),self.z.max(b.z)) }
}
impl Add for Vec3 {type Output=Self; fn add(self,b:Self)->Self{Self::new(self.x+b.x,self.y+b.y,self.z+b.z)}}
impl Sub for Vec3 {type Output=Self; fn sub(self,b:Self)->Self{Self::new(self.x-b.x,self.y-b.y,self.z-b.z)}}
impl Mul<f32> for Vec3 {type Output=Self; fn mul(self,s:f32)->Self{Self::new(self.x*s,self.y*s,self.z*s)}}
impl Div<f32> for Vec3 {type Output=Self; fn div(self,s:f32)->Self{self*(1.0/s)}}
impl Neg for Vec3 {type Output=Self; fn neg(self)->Self{Self::new(-self.x,-self.y,-self.z)}}
