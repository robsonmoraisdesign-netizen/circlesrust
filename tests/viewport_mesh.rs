use circlesrust::{integrator::CameraSettings,mesh_view::*,scene::{loader::{load_scene,SceneLoadOptions},selection::{pick,project_point},transform::ObjectTransform},util::math::Vec3};
#[test]
fn projection_matches_picking_across_camera_orbits(){
    let scene=load_scene(&SceneLoadOptions::studio()).unwrap().scene;
    for angle in [0.0_f32,0.5,1.4] {
        let mut camera=CameraSettings::default();
        circlesrust::navigation::orbit(&mut camera,angle*100.0,angle*20.0);
        for aspect in [0.7,1.0,1.7778] {
            let matrix=view_projection(camera,aspect).unwrap();
            for y in 1..10 {for x in 1..10 {
                let (u,v)=(x as f32/10.0,y as f32/10.0);
                if let Some(hit)=pick(&scene,camera,u,v,aspect){
                    let p=matrix.apply(hit.hit_position);
                    assert!(((p[0]/p[3]+1.0)*0.5-u).abs()<1e-4);
                    assert!(((1.0-p[1]/p[3])*0.5-v).abs()<1e-4);
                    assert!((0.0..=1.0).contains(&(p[2]/p[3])));
                    let projected=project_point(camera,hit.hit_position,aspect).unwrap();
                    assert!((projected.0-u).abs()<1e-4);
                }
            }}
        }
    }
    assert!(view_projection(CameraSettings::default(),f32::NAN).is_none());
}
#[test]
fn gpu_model_and_normals_match_committed_geometry_without_reupload(){
    let mut scene=load_scene(&SceneLoadOptions::studio()).unwrap().scene;
    let original=mesh_bytes(&scene);
    let id=scene.triangle_objects[0];
    let t=ObjectTransform{translation:Vec3::new(2.0,-3.0,0.4),rotation_degrees:Vec3::new(20.0,35.0,-50.0),scale:Vec3::new(0.3,1.7,2.4)};
    let (m,n)=object_matrices(t,scene.object_pivots[id]);
    let preview=transform_bytes(&scene,Some((id,t)));
    scene.apply_object_transform(id,t).unwrap();
    assert_eq!(preview,transform_bytes(&scene,None));
    assert_eq!(original,mesh_bytes(&scene));
    for (i,base) in scene.base_triangles.iter().enumerate().filter(|(i,_)|scene.triangle_objects[*i]==id){
        let actual=scene.bvh.triangle(i).unwrap();
        for (p,q) in [base.a,base.b,base.c].into_iter().zip([actual.a,actual.b,actual.c]){
            let got=m.apply(p);assert!((Vec3::new(got[0],got[1],got[2])-q).length_squared()<1e-9);
        }
        let base_normal=(base.b-base.a).cross(base.c-base.a).normalized();
        let actual_normal=(actual.b-actual.a).cross(actual.c-actual.a).normalized();
        if let (Some(b),Some(a))=(base_normal,actual_normal){
            let v=n.apply(b);let got=Vec3::new(v[0],v[1],v[2]).normalized().unwrap();
            assert!(got.dot(a)>0.999);
        }
    }
}
