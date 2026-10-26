//! Incremental Rust translation of Blender Cycles components.
//! Static-triangle CPU BVH2 intersections are implemented.
//! This is not yet the Cycles path tracer, material shader engine or GPU backend.
pub mod bvh;
pub mod kernel;
pub mod util;
