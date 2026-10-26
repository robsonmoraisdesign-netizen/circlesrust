use circlesrust::scene::{obj::ObjMesh, obj_studio_scene};
use circlesrust::integrator::{render_rgb_denoised, RenderSettings};
use circlesrust::util::math::Vec3;

#[test]
fn reads_quad_triangulation_and_normals(){
    let obj=ObjMesh::parse(include_str!("fixtures/pyramid.obj")).unwrap();
    assert_eq!(obj.triangles.len(),6);
    assert_eq!(obj.corner_normals.len(),6);
    assert!(obj.corner_normals.iter().all(|x|x.is_some()));
    let scene=obj_studio_scene(obj);
    assert_eq!(scene.bvh.stats().triangles,8); // ground 2 + pyramid 6
    assert_eq!(scene.corner_normals.len(),8);
    let s=RenderSettings{width:20,height:15,samples:2,max_bounces:3,exposure:1.0,seed:30};
    let rgb=render_rgb_denoised(&scene,s,2).unwrap();
    assert_eq!(rgb.len(),20*15*3);
}

#[test]
fn supports_negative_face_indices(){
    let src="v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf -4 -3 -2 -1\n";
    let mesh=ObjMesh::parse(src).unwrap();
    assert_eq!(mesh.triangles.len(),2);
    assert!(mesh.corner_normals.iter().all(|x|x.is_none()));
}

#[test]
fn invalid_mesh_data_is_rejected(){
    assert!(ObjMesh::parse("v 0 0 0\nf 1 2 3\n").is_err());
    assert!(ObjMesh::parse("v 0 0 inf\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").is_err());
    assert!(ObjMesh::parse("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2\n").is_err());
    assert!(ObjMesh::parse("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 nope\n").is_err());
}
#[test]
fn scene_preserves_interpolated_normals_from_obj(){
    let mesh=ObjMesh::parse(include_str!("fixtures/pyramid.obj")).unwrap();
    let scene=obj_studio_scene(mesh);
    let face=scene.bvh.triangle(2).unwrap();
    let smooth=scene.shading_normal(2,0.2,0.3);
    assert!(smooth.dot(face.normal())>0.0);
    assert!(smooth!=Vec3::ZERO);
}
