// SPDX-License-Identifier: Apache-2.0
//! Screen-space world-axis handles. Independent of egui and GPU APIs.
use crate::{desktop_transform::{EditAxis, EditTool}, integrator::CameraSettings,
    scene::{selection::project_point, transform::ObjectTransform}, util::math::Vec3};

#[derive(Clone, Copy, Debug)]
pub struct AxisHandle {
    pub axis: EditAxis,
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub world_length: f32,
}

fn vector(axis: EditAxis) -> Vec3 {
    match axis {
        EditAxis::X => Vec3::new(1.0, 0.0, 0.0),
        EditAxis::Y => Vec3::new(0.0, 1.0, 0.0),
        EditAxis::Z => Vec3::new(0.0, 0.0, 1.0),
        EditAxis::Free => Vec3::ZERO,
    }
}

/// Return projected handles in pixels relative to the actual rendered image.
/// Nearly view-aligned axes are omitted because dragging them is ambiguous.
pub fn handles(camera: CameraSettings, pivot: Vec3, size: [f32; 2], length: f32) -> Vec<AxisHandle> {
    if size.iter().any(|v| !v.is_finite() || *v < 1.0) || !length.is_finite() || length <= 0.0 {
        return Vec::new();
    }
    let Some(forward) = (camera.target - camera.origin).normalized() else { return Vec::new(); };
    let depth = (pivot - camera.origin).dot(forward);
    let world_length = length * 2.0 * depth * (camera.fov_degrees.to_radians() * 0.5).tan() / size[1];
    let aspect = size[0] / size[1];
    let Some((u, v)) = project_point(camera, pivot, aspect) else { return Vec::new(); };
    let start = [u * size[0], v * size[1]];
    [EditAxis::X, EditAxis::Y, EditAxis::Z].into_iter().filter_map(|axis| {
        let (u, v) = project_point(camera, pivot + vector(axis) * world_length, aspect)?;
        let end = [u * size[0], v * size[1]];
        let squared = (end[0] - start[0]).powi(2) + (end[1] - start[1]).powi(2);
        (squared >= 64.0).then_some(AxisHandle { axis, start, end, world_length })
    }).collect()
}

/// Nearest axis wins; the shared center is excluded to avoid ambiguous picks.
pub fn hit_test(handles: &[AxisHandle], point: [f32; 2], tolerance: f32) -> Option<AxisHandle> {
    if !tolerance.is_finite() || tolerance <= 0.0 || point.iter().any(|v| !v.is_finite()) { return None; }
    handles.iter().filter_map(|h| {
        let d = [h.end[0] - h.start[0], h.end[1] - h.start[1]];
        let len2 = d[0] * d[0] + d[1] * d[1];
        if len2 < 64.0 { return None; }
        let t = ((point[0] - h.start[0]) * d[0] + (point[1] - h.start[1]) * d[1]) / len2;
        if t < 0.18 { return None; }
        let t = t.min(1.0);
        let distance = (point[0] - h.start[0] - t * d[0]).powi(2)
            + (point[1] - h.start[1] - t * d[1]).powi(2);
        (distance <= tolerance * tolerance).then_some((distance, *h))
    }).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, h)| h)
}

/// Use total pointer displacement and the original transform, avoiding drift.
/// Scale grows along the displayed axis, including horizontal handles.
pub fn drag(initial: ObjectTransform, tool: EditTool, handle: AxisHandle, delta: [f32; 2]) -> Option<ObjectTransform> {
    if delta.iter().any(|v| !v.is_finite()) || handle.axis == EditAxis::Free { return None; }
    let d = [handle.end[0] - handle.start[0], handle.end[1] - handle.start[1]];
    let len2 = d[0] * d[0] + d[1] * d[1];
    if !len2.is_finite() || len2 < 64.0 { return None; }
    let amount = (delta[0] * d[0] + delta[1] * d[1]) / len2;
    let mut result = initial;
    match tool {
        EditTool::Move => result.translation = initial.translation + vector(handle.axis) * (amount * handle.world_length),
        EditTool::Scale => {
            let factor = amount.clamp(-9.0, 9.0).exp();
            match handle.axis {
                EditAxis::X => result.scale.x = (initial.scale.x * factor).clamp(0.01, 100.0),
                EditAxis::Y => result.scale.y = (initial.scale.y * factor).clamp(0.01, 100.0),
                EditAxis::Z => result.scale.z = (initial.scale.z * factor).clamp(0.01, 100.0),
                EditAxis::Free => return None,
            }
        }
        _ => return None,
    }
    result.valid().then_some(result)
}
