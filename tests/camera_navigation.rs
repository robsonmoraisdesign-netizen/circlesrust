use circlesrust::integrator::CameraSettings;
use circlesrust::navigation::{pan,orbit,zoom};
use circlesrust::util::math::Vec3;

fn near(a:f32,b:f32){
    assert!((a-b).abs()<1e-4,"expected {a} approximately {b}");
}
#[test]
fn pan_moves_camera_and_target_without_changing_view_direction(){
    let mut camera=CameraSettings::default();
    let initial=camera;
    let d0=camera.target-camera.origin;
    assert!(pan(&mut camera,120.0,-50.0,500.0));
    let d1=camera.target-camera.origin;
    near(d0.x,d1.x);near(d0.y,d1.y);near(d0.z,d1.z);
    let moved_origin=camera.origin-initial.origin;
    let moved_target=camera.target-initial.target;
    near(moved_origin.x,moved_target.x);
    near(moved_origin.y,moved_target.y);
    near(moved_origin.z,moved_target.z);
    assert!(moved_origin.length_squared()>0.0001);
}
#[test]
fn pan_is_proportional_to_distance_and_inverse_height(){
    let a=CameraSettings::default();
    let mut near_camera=a;let mut far_camera=a;let mut tall=a;
    far_camera.origin=far_camera.target+(far_camera.origin-far_camera.target)*2.0;
    assert!(pan(&mut near_camera,80.0,0.0,600.0));
    assert!(pan(&mut far_camera,80.0,0.0,600.0));
    assert!(pan(&mut tall,80.0,0.0,1200.0));
    let a_move=(near_camera.origin-a.origin).length_squared().sqrt();
    let far_move=(far_camera.origin-
        (a.target+(a.origin-a.target)*2.0)).length_squared().sqrt();
    let tall_move=(tall.origin-a.origin).length_squared().sqrt();
    near(far_move,2.0*a_move);
    near(tall_move,a_move*0.5);
}
#[test]
fn near_vertical_camera_still_pans(){
    let mut camera=CameraSettings{
        origin:Vec3::new(0.0,0.0,10.0),
        target:Vec3::ZERO,
        fov_degrees:45.0,
    };
    assert!(pan(&mut camera,40.0,20.0,600.0));
    assert!(camera.origin.is_finite());
    assert!(camera.target.is_finite());
    near((camera.target-camera.origin).length_squared(),100.0);
}
#[test]
fn orbit_and_zoom_remain_available(){
    let mut camera=CameraSettings::default();
    let radius=(camera.origin-camera.target).length_squared().sqrt();
    assert!(orbit(&mut camera,30.0,20.0));
    near((camera.origin-camera.target).length_squared().sqrt(),radius);
    assert!(zoom(&mut camera,120.0));
    assert!((camera.origin-camera.target).length_squared().sqrt()<radius);
    let preserved=camera;
    assert!(!pan(&mut camera,5.0,5.0,0.0));
    assert_eq!(camera.origin,preserved.origin);
}
