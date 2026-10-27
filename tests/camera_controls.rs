use circlesrust::integrator::{
    CameraSettings,RenderSettings,render_rgb,render_rgb_with_camera,
    render_rgb_denoised_with_camera
};
use circlesrust::scene::studio_scene;
use circlesrust::util::math::Vec3;

#[test]
fn default_camera_keeps_legacy_pixels(){
    let scene=studio_scene();
    let options=RenderSettings{width:24,height:16,samples:3,max_bounces:3,exposure:1.0,seed:98};
    let a=render_rgb(&scene,options).unwrap();
    let b=render_rgb_with_camera(&scene,options,CameraSettings::default()).unwrap();
    assert_eq!(a,b);
    let changed=render_rgb_with_camera(&scene,options,CameraSettings{
        origin:Vec3::new(-3.7,-6.0,4.3),..CameraSettings::default()
    }).unwrap();
    assert_ne!(a,changed);
    let denoised=render_rgb_denoised_with_camera(
        &scene,options,CameraSettings::default(),2
    ).unwrap();
    assert_eq!(a.len(),denoised.len());
}
#[test]
fn camera_rejects_invalid_or_zero_length_view(){
    let mut camera=CameraSettings::default();
    camera.target=camera.origin;
    assert!(camera.validate().is_err());
    camera=CameraSettings::default();
    camera.fov_degrees=f32::NAN;
    assert!(camera.validate().is_err());
    camera=CameraSettings::default();
    camera.fov_degrees=180.0;
    assert!(camera.validate().is_err());
}
