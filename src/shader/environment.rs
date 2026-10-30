// SPDX-License-Identifier: Apache-2.0
//! Native Rust importance sampling inspired by Cycles background_map_sample/pdf
//! and scene/light.cpp background_cdf. Not a port of portals or light trees.
use super::ImageTexture;
use crate::util::math::Vec3;
use std::f64::consts::{PI, TAU};

/// Cached probability mass per spherical cell. Radiance stays full resolution.
#[derive(Debug)]
pub struct EnvironmentDistribution {
    width: usize,
    height: usize,
    cdf: Vec<f64>,
}
impl EnvironmentDistribution {
    pub fn new(map: &ImageTexture) -> Self {
        let width = map.width.min(512);
        let height = map.height.min(256);
        let mut weights = vec![0.0; width * height];
        // Accumulate every source texel so downsampling does not skip small suns.
        // f64 keeps finite HDR values from overflowing while forming the CDF.
        for y in 0..map.height {
            let area = cell_area(y, map.width, map.height);
            for x in 0..map.width {
                let p = map.pixel(x, y);
                let luminance = 0.2126 * p.x as f64 + 0.7152 * p.y as f64 + 0.0722 * p.z as f64;
                weights[(y * height / map.height) * width + x * width / map.width] += luminance * area;
            }
        }
        let total: f64 = weights.iter().sum();
        let mut cdf = Vec::with_capacity(weights.len() + 1);
        cdf.push(0.0);
        for (index, weight) in weights.into_iter().enumerate() {
            let uniform = cell_area(index / width, width, height) / (4.0 * PI);
            // Nonzero support everywhere is essential: bilinear texture filtering
            // can make a direction bright even if its own source texel is black.
            let probability = if total > 0.0 { 0.95 * weight / total + 0.05 * uniform } else { uniform };
            cdf.push(cdf.last().copied().unwrap() + probability);
        }
        let sum = *cdf.last().unwrap();
        for value in &mut cdf { *value /= sum; }
        Self { width, height, cdf }
    }

    /// Three independent uniform variates: cell, azimuth within cell, cos(theta).
    pub fn sample(&self, random: [f32; 3]) -> Option<(Vec3, f32)> {
        if random.iter().any(|r| !r.is_finite() || !(0.0..=1.0).contains(r)) { return None; }
        let r = random.map(|v| (v as f64).min(1.0 - f64::EPSILON));
        let index = self.cdf.partition_point(|value| *value <= r[0])
            .saturating_sub(1).min(self.width * self.height - 1);
        let x = index % self.width;
        let y = index / self.width;
        let phi = TAU * ((x as f64 + r[1]) / self.width as f64 - 0.5);
        let top = (PI * y as f64 / self.height as f64).cos();
        let bottom = (PI * (y + 1) as f64 / self.height as f64).cos();
        let z = top + r[2] * (bottom - top);
        let radius = (1.0 - z * z).max(0.0).sqrt();
        let direction = Vec3::new((radius * phi.cos()) as f32, (radius * phi.sin()) as f32, z as f32);
        Some((direction, self.pdf(direction)))
    }

    /// Density per steradian, identical for direct-light and BSDF MIS paths.
    /// Exact cell solid angles avoid division by sin(theta) at the poles.
    pub fn pdf(&self, direction: Vec3) -> f32 {
        let Some(d) = direction.normalized() else { return 0.0; };
        let u = (0.5 + (d.y as f64).atan2(d.x as f64) / TAU).rem_euclid(1.0);
        let v = (d.z as f64).clamp(-1.0, 1.0).acos() / PI;
        let x = ((u * self.width as f64) as usize).min(self.width - 1);
        let y = ((v * self.height as f64) as usize).min(self.height - 1);
        let index = y * self.width + x;
        ((self.cdf[index + 1] - self.cdf[index]) / cell_area(y, self.width, self.height)) as f32
    }
}
fn cell_area(row: usize, width: usize, height: usize) -> f64 {
    TAU / width as f64 * ((PI * row as f64 / height as f64).cos()
        - (PI * (row + 1) as f64 / height as f64).cos())
}
