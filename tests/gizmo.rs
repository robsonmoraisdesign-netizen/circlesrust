use circlesrust::{desktop_transform::{EditAxis,EditTool}, gizmo::{self,AxisHandle},
    integrator::CameraSettings, scene::transform::ObjectTransform, util::math::Vec3};
fn handle()->AxisHandle { AxisHandle{axis:EditAxis::X,start:[10.0,20.0],end:[90.0,20.0],world_length:2.0} }
#[test]
fn picking_selects_nearest_axis_and_excludes_center(){
    let x=handle();
    let y=AxisHandle{axis:EditAxis::Y,end:[10.0,100.0],..x};
    assert_eq!(gizmo::hit_test(&[x,y],[85.0,22.0],9.0).unwrap().axis,EditAxis::X);
    assert_eq!(gizmo::hit_test(&[x,y],[12.0,85.0],9.0).unwrap().axis,EditAxis::Y);
    assert!(gizmo::hit_test(&[x,y],[10.0,20.0],9.0).is_none());
    assert!(gizmo::hit_test(&[x,y],[110.0,20.0],9.0).is_none());
    assert!(gizmo::hit_test(&[x],[f32::NAN,0.0],9.0).is_none());
}
#[test]
fn horizontal_scale_and_move_follow_axis_without_cross_axis_drift(){
    let initial=ObjectTransform::default();
    let moved=gizmo::drag(initial,EditTool::Move,handle(),[40.0,70.0]).unwrap();
    assert_eq!(moved.translation,Vec3::new(1.0,0.0,0.0));
    let scaled=gizmo::drag(initial,EditTool::Scale,handle(),[40.0,0.0]).unwrap();
    assert!(scaled.scale.x>1.0);
    assert_eq!(scaled.scale.y,1.0);
    assert_eq!(scaled.scale.z,1.0);
    assert_eq!(gizmo::drag(initial,EditTool::Move,handle(),[0.0,0.0]).unwrap(),initial);
    assert!(gizmo::drag(initial,EditTool::Move,handle(),[f32::INFINITY,0.0]).is_none());
}
#[test]
fn projected_handles_work_with_orbit_zoom_and_non_square_images(){
    let mut camera=CameraSettings::default();
    for size in [[800.0,450.0],[450.0,800.0]] {
        for _ in 0..6 {
            circlesrust::navigation::orbit(&mut camera,45.0,10.0);
            circlesrust::navigation::zoom(&mut camera,5.0);
            let handles=gizmo::handles(camera,camera.target,size,82.0);
            assert!(handles.len()>=2);
            for h in handles {
                let d=[h.end[0]-h.start[0],h.end[1]-h.start[1]];
                let changed=gizmo::drag(ObjectTransform::default(),EditTool::Move,h,d).unwrap();
                let distance=changed.translation.length_squared().sqrt();
                assert!((distance-h.world_length).abs()<1e-4);
            }
        }
    }
    assert!(gizmo::handles(camera,camera.origin,[800.0,450.0],82.0).is_empty());
    assert!(gizmo::handles(camera,camera.target,[0.0,450.0],82.0).is_empty());
}
