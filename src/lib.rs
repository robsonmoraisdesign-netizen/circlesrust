//! Partial Cycles-to-Rust CPU renderer: BVH, ray intersections and sampled BSDF path integration.
//! This is not yet the complete Cycles engine or a GPU device backend.
pub mod bvh;
pub mod integrator;
pub mod kernel;
pub mod util;
