use circlesrust::scene::{obj::ObjMesh,obj_studio_scene,studio_scene,selection::{pick,object_bounds,frame_object}};
use circlesrust::integrator::CameraSettings;
use circlesrust::util::math::Vec3;
fn v(x:f32,y:f32,z:f32)->Vec3{Vec3::new(x,y,z)}
#[test]
fn obj_group_names_survive_triangulation_and_are_not_materials(){
    let source="v -2 0 0\nv -1 0 0\nv -2 1 0\nv 1 0 0\nv 2 0 0\nv 1 1 0\n\
o LeftPart\nusemtl SameMaterial\nf 1 2 3\n\
o RightPart\nusemtl SameMaterial\nf 4 5 6";
    let mesh=ObjMesh::parse(source).unwrap();
    assert_eq!(mesh.face_groups,vec!["LeftPart","RightPart"]);
    let scene=obj_studio_scene(mesh);
    assert_eq!(scene.triangle_objects.len(),scene.bvh.stats().triangles);
    assert_ne!(scene.triangle_objects[2],scene.triangle_objects[3]);
    assert_eq!(scene.triangle_materials[2],scene.triangle_materials[3]);
}
#[test]
fn pick_actual_triangle_and_frame_camera_without_changing_object(){
    let scene=studio_scene();
    let camera=CameraSettings{
        origin:v(0.8,-5.0,2.0),
        target:v(0.8,-1.04,0.66),
        fov_degrees:40.0,
    };
    let hit=pick(&scene,camera,0.5,0.5,1.5).expect("sphere near the center");
    assert!(hit.object_id<scene.object_names.len());
    assert_eq!(hit.material_id,scene.triangle_materials[hit.triangle_id]);
    assert!(hit.hit_position.is_finite());
    let mut other=camera;
    assert!(frame_object(&scene,&mut other,hit.object_id,1.5));
    let bounds=object_bounds(&scene,hit.object_id).unwrap();
    let d=(other.target-bounds.center()).length_squared().sqrt();
    assert!(d<0.00001);
    assert!(other.validate().is_ok());
}
#[test]
fn picking_rejects_invalid_coordinates_and_empty_groups(){
    let scene=studio_scene();
    let c=CameraSettings::default();
    assert!(pick(&scene,c,f32::NAN,0.5,1.0).is_none());
    assert!(pick(&scene,c,1.1,0.5,1.0).is_none());
    assert!(object_bounds(&scene,scene.object_names.len()+5).is_none());
    let mut camera=c;
    assert!(!frame_object(&scene,&mut camera,999,1.0));
    assert_eq!(camera.origin,c.origin);
}
