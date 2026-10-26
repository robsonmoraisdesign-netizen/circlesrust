use circlesrust::bvh::Bvh;
use circlesrust::kernel::geometry::{triangle_intersect,Ray,Triangle};
use circlesrust::scene::{obj::ObjMesh,mtl::parse_mtl,obj_studio_scene_with_materials};
use circlesrust::util::math::Vec3;

fn v(x:f32,y:f32,z:f32)->Vec3 {Vec3::new(x,y,z)}
fn ray(x:f32,y:f32)->Ray {
    Ray::new(v(x,y,-2.0),v(0.0,0.0,1.0),0.001,100.0).unwrap()
}
#[test]
fn obj_mtl_materials_are_selected_per_face(){
    let mesh=ObjMesh::parse(include_str!("fixtures/colored.obj")).unwrap();
    assert_eq!(mesh.triangles.len(),4);
    assert_eq!(mesh.face_materials.len(),4);
    assert_eq!(mesh.material_libraries,vec!["colored.mtl"]);
    assert_eq!(mesh.face_materials[0].as_deref(),Some("matte_red"));
    assert_eq!(mesh.face_materials[1].as_deref(),Some("matte_red"));
    assert_eq!(mesh.face_materials[2].as_deref(),Some("metallic_blue"));
    let materials=parse_mtl(include_str!("fixtures/colored.mtl")).unwrap();
    assert_eq!(materials.len(),2);
    assert_eq!(materials[0].name,"matte_red");
    assert_eq!(materials[1].name,"metallic_blue");
    assert!((materials[1].material.metallic-0.85).abs()<0.00001);
    assert!((materials[1].material.roughness-0.16).abs()<0.00001);
    let scene=obj_studio_scene_with_materials(mesh,&materials);
    // The first two triangles are the ground, then two red and two blue.
    assert_eq!(&scene.triangle_materials[..6], &[0,0,2,2,3,3]);
    assert_eq!(scene.materials.len(),4);
    assert!(scene.materials[2].base_color.x>scene.materials[2].base_color.y);
    assert!(scene.materials[3].base_color.z>scene.materials[3].base_color.x);
}
#[test]
fn mtl_phong_ns_converts_to_roughness_and_validates_values(){
    let m=parse_mtl("newmtl sample\nKd 0.4 0.3 0.2\nNs 98\n").unwrap();
    assert!((m[0].material.roughness-(0.02f32).sqrt()).abs()<0.00001);
    assert!(parse_mtl("newmtl bad\nPr NaN").is_err());
    assert!(parse_mtl("newmtl bad\nKd 0.5 0.2").is_err());
    assert!(parse_mtl("Kd 0.3 0.4 0.5").is_err());
    assert!(parse_mtl("newmtl").is_err());
}
#[test]
fn binned_sah_matches_brute_force_for_clustered_geometry(){
    let mut tris=Vec::new();
    for side in [-24.0f32,24.0] {
        for y in 0..14 {
            for x in 0..12 {
                let px=side+x as f32*0.15;
                let py=y as f32*0.12;
                tris.push(Triangle::new(v(px,py,3.0),v(px+0.12,py,3.0),v(px,py+0.10,3.0)).unwrap());
            }
        }
    }
    let tree=Bvh::build(tris.clone());
    let stats=tree.stats();
    assert_eq!(stats.triangles,tris.len());
    assert!(stats.leaves>1);
    assert!(stats.maximum_depth>2);
    for i in 0..300 {
        let side=if i%2==0{-24.0}else{24.0};
        let x=side+((i*23)%205) as f32/100.0;
        let y=((i*61)%185) as f32/100.0;
        let beam=ray(x,y);
        let expected=tris.iter()
            .filter_map(|t|triangle_intersect(&beam,t,beam.t_max).map(|h|h.0))
            .min_by(f32::total_cmp);
        let actual=tree.closest_hit(&beam).map(|hit|hit.t);
        assert_eq!(expected.is_some(),actual.is_some(),"ray {i} mismatch");
        if let (Some(a),Some(b))=(expected,actual){assert!((a-b).abs()<1e-4);}
    }
}
#[test]
fn same_centroid_has_median_fallback(){
    let tris=(0..32).map(|i|{
        let scale=(i+1) as f32/32.0;
        Triangle::new(v(-scale,-scale,3.0),v(scale,-scale,3.0),
            v(0.0,2.0*scale,3.0)).unwrap()
    }).collect();
    let tree=Bvh::build(tris);
    assert_eq!(tree.stats().triangles,32);
    assert!(tree.stats().leaves>=4);
    assert!(tree.occluded(&ray(0.0,0.0)));
}
