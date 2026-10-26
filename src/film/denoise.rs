// SPDX-License-Identifier: Apache-2.0
//! Deterministic, optional guided a-trous denoising of display RGB pixels.
//! This is NOT Intel OIDN, OptiX AI denoising, or Cycles adaptive sampling.
//! Normals and first-hit depth/material segmentation suppress cross-edge bleed.
use crate::util::math::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct PixelGuide {
    pub normal: Vec3,
    pub depth: f32,
    pub surface: Option<usize>,
}
impl PixelGuide {
    pub const SKY: Self = Self { normal: Vec3::ZERO, depth: f32::INFINITY, surface: None };
}

/// Runs a small number of edge-aware 5x5 a-trous passes on an RGB image.
/// The same film values and guides always produce byte-identical results.
/// Strength 1..=5 enables filtering, and zero bypasses it.
pub fn denoise_rgb(
    rgb:&[u8], width:u32, height:u32, guides:&[PixelGuide], strength:u8,
)->Result<Vec<u8>,&'static str>{
    let count=(width as usize).checked_mul(height as usize).ok_or("invalid dimensions")?;
    if width==0 || height==0 || rgb.len()!=count*3 || guides.len()!=count || strength>5 {
        return Err("invalid denoise pixels, guides or strength");
    }
    if strength==0 {return Ok(rgb.to_vec());}
    let mut current=rgb.to_vec();
    let mut scratch=vec![0u8;rgb.len()];
    let width=width as usize;
    let height=height as usize;
    let kernel=[1.0f32,4.0,6.0,4.0,1.0];
    for pass in 0..strength {
        let step=1usize<<pass;
        for y in 0..height {
            for x in 0..width {
                let i=y*width+x;
                let center=guides[i];
                let ci=i*3;
                let color=[
                    current[ci] as f32,
                    current[ci+1] as f32,
                    current[ci+2] as f32,
                ];
                let mut sum=[0.0f32;3];
                let mut total=0.0f32;
                for ky in 0..5usize {
                    let sy=y as isize+(ky as isize-2)*step as isize;
                    if sy<0 || sy>=height as isize {continue;}
                    for kx in 0..5usize {
                        let sx=x as isize+(kx as isize-2)*step as isize;
                        if sx<0 || sx>=width as isize {continue;}
                        let j=sy as usize*width+sx as usize;
                        let neighbor=guides[j];
                        // Do not mix sky, different object material or a background.
                        if neighbor.surface != center.surface {continue;}
                        let guide_weight=if center.surface.is_some() {
                            let ndot=center.normal.dot(neighbor.normal).clamp(-1.0,1.0);
                            if ndot<0.85 {continue;}
                            let delta=(center.depth-neighbor.depth).abs();
                            let tolerance=0.03+center.depth.max(0.0)*0.025;
                            let depth_weight=(-delta/tolerance).exp();
                            let normal_weight=((ndot-1.0)*32.0).exp();
                            depth_weight*normal_weight
                        }else{1.0};
                        let idx=j*3;
                        let variance=(0..3).map(|c| {let delta=current[idx+c] as f32-color[c];delta*delta}).sum::<f32>();
                        // Reject strong RGB discontinuities independent of geometry.
                        let color_weight=(-variance/(3.0*40.0*40.0)).exp();
                        let weight=kernel[kx]*kernel[ky]*guide_weight*color_weight;
                        total+=weight;
                        for c in 0..3{sum[c]+=current[idx+c] as f32*weight;}
                    }
                }
                for c in 0..3 {
                    scratch[ci+c]=if total>0.0 {
                        (sum[c]/total+0.5).clamp(0.0,255.0) as u8
                    }else{current[ci+c]};
                }
            }
        }
        std::mem::swap(&mut current,&mut scratch);
    }
    Ok(current)
}
