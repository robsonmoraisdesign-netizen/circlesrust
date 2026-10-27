use circlesrust::integrator::{CameraSettings,RenderSettings};
use circlesrust::scene::studio_scene;
use circlesrust::preview::solid_preview;
use circlesrust::util::math::Vec3;

#[test]
fn solid_preview_shows_geometry_without_path_trace(){
    let scene=studio_scene();
    let a=solid_preview(&scene,CameraSettings::default(),64,40).unwrap();
    assert_eq!(a.len(),64*40*3);
    assert!(a.as_chunks::<3>().0.iter().any(|v|v!=&[a[0],a[1],a[2]]));
    assert_eq!(a,solid_preview(&scene,CameraSettings::default(),64,40).unwrap());
    let changed=solid_preview(&scene,CameraSettings{
        origin:Vec3::new(-4.1,-7.6,3.65),..CameraSettings::default()
    },64,40).unwrap();
    assert_ne!(changed,a);
}
#[test]
fn invalid_preview_sizes_are_rejected(){
    let scene=studio_scene();
    assert!(solid_preview(&scene,CameraSettings::default(),0,10).is_err());
    assert!(solid_preview(&scene,CameraSettings::default(),10,3000).is_err());
    let _unused=RenderSettings::default();
}
