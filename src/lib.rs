//! Incremental native Rust rendering modules derived from Blender Cycles.
//! Current output uses a CPU path integrator, but not the full Cycles kernel/GPU.
pub mod bvh;
pub mod film;
pub mod integrator;
pub mod kernel;
pub mod scene;
pub mod util;
pub mod shader;

#[cfg(feature="desktop-ui")]
pub mod desktop;

#[path="desktop/preview.rs"]
pub mod preview;

#[path="desktop/navigation.rs"]
pub mod navigation;

#[path="desktop/transform_tools.rs"]
pub mod desktop_transform;

#[path="desktop/gizmo.rs"]
pub mod gizmo;
