use std::sync::atomic::AtomicU64;
use circlesrust::integrator::CameraSettings;
use circlesrust::util::math::Vec3;
use circlesrust::scene::{studio_scene,selection::object_bounds};
use circlesrust::preview::{solid_preview_cancellable,solid_preview_cancellable_highlight};

#[test]
fn highlight_changes_pixels_without_mutating_scene_or_final_material(){
    let scene=studio_scene();
    let before=scene.materials.clone();
    let epoch=AtomicU64::new(5);
    let camera=CameraSettings::default();
    let raw=solid_preview_cancellable(&scene,camera,60,40,&epoch,5).unwrap();
    let selected=solid_preview_cancellable_highlight(
        &scene,camera,60,40,&epoch,5,Some(2),Vec3::new(0.95,0.57,0.11)
    ).unwrap();
    assert_ne!(raw,selected);
    assert_eq!(scene.materials.len(),before.len());
    for (a,b) in scene.materials.iter().zip(before){
        assert_eq!(a.base_color,b.base_color);
    }
    assert!(object_bounds(&scene,2).is_some());
}
