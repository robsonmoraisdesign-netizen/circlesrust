// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
// Modified from blender/cycles/src/kernel/types.h
pub const OBJECT_MOTION_PASS_SIZE: u32 = 2;
pub const FILTER_TABLE_SIZE: u32 = 1024;
pub const RAMP_TABLE_SIZE: u32 = 256;
pub const SHUTTER_TABLE_SIZE: u32 = 256;
pub const THIN_FILM_TABLE_SIZE: u32 = 512;
pub const MAX_SAMPLES: u32 = 1 << 24;
pub const BSSRDF_MIN_RADIUS: f32 = 1e-8;
pub const BSSRDF_MAX_HITS: u32 = 4;
pub const BSSRDF_MAX_BOUNCES: u32 = 256;
pub const LOCAL_MAX_HITS: u32 = 4;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplingPattern {
    SobolBurley = 0,
    TabulatedSobol = 1,
    BlueNoisePure = 2,
    BlueNoiseFirst = 3,
    BlueNoiseRound = 4,
    Automatic = 5,
}
/// Cycles uses duplicate enum discriminants for different bounce contexts.
pub mod path_dimension {
    pub const PRNG_FILTER: u32 = 0;
    pub const PRNG_LENS_TIME: u32 = 1;
    pub const PRNG_TERMINATE: u32 = 0;
    pub const PRNG_LIGHT: u32 = 1;
    pub const PRNG_LIGHT_TERMINATE: u32 = 2;
    pub const PRNG_SURFACE_BSDF: u32 = 3;
    pub const PRNG_SURFACE_AO: u32 = 4;
    pub const PRNG_SURFACE_BEVEL: u32 = 5;
    pub const PRNG_SURFACE_BSDF_GUIDING: u32 = 6;
    pub const PRNG_SURFACE_RIS_GUIDING_0: u32 = 10;
    pub const PRNG_SURFACE_RIS_GUIDING_1: u32 = 11;
    pub const PRNG_VOLUME_PHASE: u32 = 3;
    pub const PRNG_VOLUME_RESERVOIR: u32 = 4;
    pub const PRNG_VOLUME_SCATTER_DISTANCE: u32 = 5;
    pub const PRNG_VOLUME_EXPANSION_ORDER: u32 = 6;
    pub const PRNG_VOLUME_SHADE_OFFSET: u32 = 7;
    pub const PRNG_VOLUME_PHASE_GUIDING_DISTANCE: u32 = 8;
    pub const PRNG_VOLUME_PHASE_GUIDING_EQUIANGULAR: u32 = 9;
    pub const PRNG_VOLUME_COLOR_CHANNEL: u32 = 4;
    pub const PRNG_VOLUME_OFFSET: u32 = 6;
    pub const PRNG_SUBSURFACE_BSDF: u32 = 0;
    pub const PRNG_SUBSURFACE_COLOR_CHANNEL: u32 = 1;
    pub const PRNG_SUBSURFACE_SCATTER_DISTANCE: u32 = 2;
    pub const PRNG_SUBSURFACE_GUIDE_STRATEGY: u32 = 3;
    pub const PRNG_SUBSURFACE_GUIDE_DIRECTION: u32 = 4;
    pub const PRNG_SUBSURFACE_DISK: u32 = 0;
    pub const PRNG_SUBSURFACE_DISK_RESAMPLE: u32 = 1;
    pub const PRNG_BAKE_VOLUME_DENSITY_EVAL: u32 = 0;
    pub const PRNG_WAVELENGTH: u32 = 12;
    pub const PRNG_BOUNCE_NUM: u32 = 16;
}
pub mod ray_visibility {
    pub const CAMERA: u32 = 1 << 0;
    pub const TRANSMIT: u32 = 1 << 1;
    pub const DIFFUSE: u32 = 1 << 2;
    pub const GLOSSY: u32 = 1 << 3;
    pub const VOLUME_SCATTER: u32 = 1 << 4;
    pub const SHADOW_OPAQUE: u32 = 1 << 5;
    pub const SHADOW_TRANSPARENT: u32 = 1 << 6;
    pub const SHADOW: u32 = SHADOW_OPAQUE | SHADOW_TRANSPARENT;
    pub const ALL: u32 = (1 << 7) - 1;
    pub const RAYCAST: u32 = 1 << 7;
    pub const OBJECT_ALL: u32 = ALL | RAYCAST;
    pub const NODE_UNALIGNED: u32 = 1 << 15;
}
