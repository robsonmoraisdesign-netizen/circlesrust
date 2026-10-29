use circlesrust::scene::{studio_scene,transform::ObjectTransform,selection::{object_bounds,pick}};
use circlesrust::integrator::{CameraSettings,RenderSettings,render_rgb_with_camera};
use circlesrust::desktop_transform::{EditAxis,EditTool,apply_drag};
use circlesrust::util::math::Vec3;
fn v(x:f32,y:f32,z:f32)->Vec3{Vec3::new(x,y,z)}
fn close(a:f32,b:f32){assert!((a-b).abs()<1e-4,"{a} vs {b}");}
#[test]
fn movement_changes_real_mesh_intersections_and_renders(){
    let mut scene=studio_scene();
    let id=4usize; // sphere
    let before=object_bounds(&scene,id).unwrap();
    let left=scene.bvh.triangle(0).unwrap().a;
    let cam=CameraSettings::default();
    let opts=RenderSettings{width:32,height:24,samples:2,max_bounces:2,exposure:1.0,seed:103};
    let raw=render_rgb_with_camera(&scene,opts,cam).unwrap();
    scene.apply_object_transform(id,ObjectTransform{
        translation:v(1.8,0.0,0.4),..Default::default()
    }).unwrap();
    let after=object_bounds(&scene,id).unwrap();
    close(after.center().x-before.center().x,1.8);
    close(after.center().z-before.center().z,0.4);
    assert_eq!(scene.bvh.triangle(0).unwrap().a,left);
    assert_ne!(raw,render_rgb_with_camera(&scene,opts,cam).unwrap());
    scene.apply_object_transform(id,ObjectTransform::default()).unwrap();
    let reset=object_bounds(&scene,id).unwrap();
    close(reset.center().x,before.center().x);
    close(reset.center().z,before.center().z);
    assert_eq!(raw,render_rgb_with_camera(&scene,opts,cam).unwrap());
}
#[test]
fn rotation_and_non_uniform_scale_keep_normals_valid(){
    let mut scene=studio_scene();
    let id=4;
    scene.apply_object_transform(id,ObjectTransform{
        translation:v(0.0,0.0,0.1),rotation_degrees:v(35.0,50.0,90.0),
        scale:v(2.0,0.7,1.3)
    }).unwrap();
    for (i,&group) in scene.triangle_objects.iter().enumerate(){
        if group==id {
            assert!(scene.bvh.triangle(i).unwrap().is_valid());
            if let Some(corners)=scene.corner_normals[i]{
                for normal in corners {
                    assert!(normal.is_finite());
                    close(normal.length_squared(),1.0);
                }
            }
        }
    }
    assert!(scene.apply_object_transform(id,ObjectTransform{
        scale:v(-1.0,1.0,1.0),..Default::default()
    }).is_err());
    assert!(scene.apply_object_transform(999,ObjectTransform::default()).is_err());
}
#[test]
fn viewport_drag_changes_selected_transform_on_constrained_axes(){
    let camera=CameraSettings::default();
    let t=ObjectTransform::default();
    let moved=apply_drag(t,EditTool::Move,EditAxis::X,60.0,20.0,camera,450.0).unwrap();
    assert_ne!(moved.translation.x,0.0);
    close(moved.translation.y,0.0);
    close(moved.translation.z,0.0);
    let rotated=apply_drag(t,EditTool::Rotate,EditAxis::Z,50.0,0.0,camera,450.0).unwrap();
    assert!(rotated.rotation_degrees.z>0.0);
    let scaled=apply_drag(t,EditTool::Scale,EditAxis::Free,0.0,-30.0,camera,450.0).unwrap();
    assert!(scaled.scale.x>1.0);
    assert!(scaled.scale.y>1.0);
    assert!(scaled.scale.z>1.0);
    assert!(apply_drag(t,EditTool::Select,EditAxis::Free,3.0,2.0,camera,450.0).is_none());
    let _=pick(&studio_scene(),camera,0.5,0.5,1.5);
}
