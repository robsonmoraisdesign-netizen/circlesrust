use std::sync::atomic::{AtomicU64,Ordering};
use circlesrust::integrator::{
    CameraSettings,RenderSettings,render_rgb_denoised_with_camera,
    render_rgb_denoised_with_camera_cancellable
};
use circlesrust::preview::{solid_preview,solid_preview_cancellable};
use circlesrust::scene::studio_scene;

fn settings()->RenderSettings{
    RenderSettings{width:16,height:12,samples:2,max_bounces:3,exposure:1.0,seed:111}
}

#[test]
fn cancellable_live_renders_same_pixels_when_unchanged(){
    let scene=studio_scene();
    let epoch=AtomicU64::new(7);
    let default=CameraSettings::default();
    let ordinary=render_rgb_denoised_with_camera(&scene,settings(),default,0).unwrap();
    let checked=render_rgb_denoised_with_camera_cancellable(
        &scene,settings(),default,0,&epoch,7
    ).unwrap();
    assert_eq!(ordinary,checked);
}
#[test]
fn obsolete_live_preview_stops_on_revision_change(){
    let scene=studio_scene();
    let epoch=AtomicU64::new(42);
    assert!(render_rgb_denoised_with_camera_cancellable(
        &scene,settings(),CameraSettings::default(),0,&epoch,41
    ).is_err());
    epoch.store(43,Ordering::Relaxed);
    assert!(render_rgb_denoised_with_camera_cancellable(
        &scene,settings(),CameraSettings::default(),0,&epoch,43
    ).is_ok());
}
#[test]
fn solid_preview_cancellation_retains_legacy_pixels(){
    let scene=studio_scene();
    let camera=CameraSettings::default();
    let epoch=AtomicU64::new(1);
    let expected=solid_preview(&scene,camera,24,16).unwrap();
    let actual=solid_preview_cancellable(&scene,camera,24,16,&epoch,1).unwrap();
    assert_eq!(expected,actual);
    epoch.store(2,Ordering::Relaxed);
    assert!(solid_preview_cancellable(&scene,camera,24,16,&epoch,1).is_err());
}
