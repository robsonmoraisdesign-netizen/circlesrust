// SPDX-License-Identifier: Apache-2.0
//! First native Rust texture/shader graph milestone, not the Cycles SVM/OSL engine.
//! Uses linear RGB for shader math; 8-bit PNG/JPEG/PNM is decoded from sRGB.

use crate::util::math::Vec3;
use std::path::Path;
pub mod surface;


fn linear(channel: u8) -> f32 {
    let x = channel as f32 / 255.0;
    if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
}
fn multiply(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uv { pub u: f32, pub v: f32 }
impl Uv {
    pub const fn new(u: f32, v: f32) -> Self { Self { u, v } }
    pub fn interpolate(corners: [Uv; 3], b: f32, c: f32) -> Self {
        let a = 1.0 - b - c;
        Self { u: corners[0].u*a + corners[1].u*b + corners[2].u*c,
               v: corners[0].v*a + corners[1].v*b + corners[2].v*c }
    }
    pub fn valid(self) -> bool { self.u.is_finite() && self.v.is_finite() }
}

#[derive(Clone, Debug)]
pub struct ImageTexture {
    width: usize,
    height: usize,
    pixels: Vec<Vec3>,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TextureColorSpace { Srgb, NonColor }

impl ImageTexture {
    pub fn from_rgb8(width: u32, height: u32, pixels: &[u8]) -> Result<Self, String> {
        Self::from_rgb8_with_space(width,height,pixels,TextureColorSpace::Srgb)
    }
    /// NonColor keeps channel bytes linear for scalar/normal PBR maps.
    pub fn from_rgb8_with_space(
        width:u32,height:u32,pixels:&[u8],space:TextureColorSpace
    )->Result<Self,String>{
        let size=(width as usize).checked_mul(height as usize)
            .ok_or("Texture dimensions overflow")?;
        if size==0 || pixels.len()!=size*3 || width>16384 || height>16384 {
            return Err("Invalid texture dimensions or pixel buffer".to_owned());
        }
        let channel=|byte:u8|match space{
            TextureColorSpace::Srgb=>linear(byte),
            TextureColorSpace::NonColor=>byte as f32/255.0,
        };
        let converted=pixels.as_chunks::<3>().0.iter()
            .map(|p| Vec3::new(channel(p[0]),channel(p[1]),channel(p[2]))).collect();
        Ok(Self{width:width as usize,height:height as usize,pixels:converted})
    }
    pub fn load(path:impl AsRef<Path>) -> Result<Self,String>{
        Self::load_with_space(path,TextureColorSpace::Srgb)
    }
    pub fn load_with_space(
        path:impl AsRef<Path>,space:TextureColorSpace
    )->Result<Self,String> {
        let source=path.as_ref();
        // Decode one time during scene loading, never inside the ray loop.
        let data=image::open(source)
            .map_err(|e|format!("Cannot decode {}: {e}",source.display()))?
            .to_rgb8();
        Self::from_rgb8_with_space(data.width(),data.height(),data.as_raw(),space)
    }
    fn pixel(&self,x:usize,y:usize)->Vec3 {self.pixels[y*self.width+x]}
    /// Bilinear filtered, repeating U/V. Image pixels top-down, OBJ V bottom-up.
    pub fn sample(&self,uv:Uv)->Vec3 {
        if !uv.valid(){return Vec3::ZERO;}
        let u=uv.u.rem_euclid(1.0);
        let v=(1.0-uv.v).rem_euclid(1.0);
        let x=u*self.width as f32 - 0.5;
        let y=v*self.height as f32 - 0.5;
        let x0=x.floor() as isize;
        let y0=y.floor() as isize;
        let tx=x-x.floor();
        let ty=y-y.floor();
        let wrap=|a:isize,n:usize|a.rem_euclid(n as isize) as usize;
        let c00=self.pixel(wrap(x0,self.width),wrap(y0,self.height));
        let c10=self.pixel(wrap(x0+1,self.width),wrap(y0,self.height));
        let c01=self.pixel(wrap(x0,self.width),wrap(y0+1,self.height));
        let c11=self.pixel(wrap(x0+1,self.width),wrap(y0+1,self.height));
        let a=c00*(1.0-tx)+c10*tx;
        let b=c01*(1.0-tx)+c11*tx;
        a*(1.0-ty)+b*ty
    }
}

#[derive(Clone, Debug)]
pub enum ColorNode {
    Constant(Vec3),
    Image(ImageTexture),
    Checker { first: Vec3, second: Vec3, scale: f32 },
    Multiply { left: usize, right: usize },
    Mix { left: usize, right: usize, factor: f32 },
}

/// Forward-only graph: a node may reference only previously defined nodes.
/// This makes evaluation deterministic and avoids cycles/recursion.
#[derive(Clone, Debug)]
pub struct ShaderGraph {
    pub nodes: Vec<ColorNode>,
    pub output: usize,
}
impl ShaderGraph {
    pub fn new(nodes: Vec<ColorNode>, output: usize) -> Result<Self, &'static str> {
        if nodes.is_empty() || output>=nodes.len() || nodes.len()>64 {
            return Err("Invalid shader graph size/output");
        }
        for (i,node) in nodes.iter().enumerate() {
            match node {
                ColorNode::Multiply{left,right}|ColorNode::Mix{left,right,..}
                    if *left>=i || *right>=i =>return Err("Cycle or forward shader reference"),
                ColorNode::Constant(c) if !c.is_finite()=>return Err("Non-finite shader color"),
                ColorNode::Checker{first,second,scale}
                    if !first.is_finite() || !second.is_finite() || !scale.is_finite()
                       || *scale<=0.0=>return Err("Invalid checker"),
                ColorNode::Mix{factor,..} if !factor.is_finite()=>return Err("Invalid mix"),
                _=>{}
            }
        }
        Ok(Self{nodes,output})
    }
    pub fn evaluate(&self,uv:Uv)->Vec3{
        let mut computed=[Vec3::ZERO;64];
        for (i,node) in self.nodes.iter().enumerate() {
            computed[i]=match node {
                ColorNode::Constant(c)=>*c,
                ColorNode::Image(texture)=>texture.sample(uv),
                ColorNode::Checker{first,second,scale}=>{
                    let u=(uv.u*scale).floor() as i64;
                    let v=(uv.v*scale).floor() as i64;
                    if (u+v)&1==0{*first}else{*second}
                }
                ColorNode::Multiply{left,right}=>multiply(computed[*left],computed[*right]),
                ColorNode::Mix{left,right,factor}=>{
                    let t=factor.clamp(0.0,1.0);
                    computed[*left]*(1.0-t)+computed[*right]*t
                }
            };
        }
        computed[self.output]
    }
    /// MTL's diffuse map modulates Kd instead of replacing it.
    pub fn diffuse_map(base_color:Vec3, image:ImageTexture)->Self{
        Self::new(vec![
            ColorNode::Constant(base_color),
            ColorNode::Image(image),
            ColorNode::Multiply{left:0,right:1},
        ],2).expect("valid constant-image-multiply graph")
    }
}
