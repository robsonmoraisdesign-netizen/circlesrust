use circlesrust::bvh::{Aabb,Bvh,MAX_LEAF_TRIANGLES};
use circlesrust::kernel::geometry::{triangle_intersect,Ray,Triangle};
use circlesrust::util::math::Vec3;

fn v(x:f32,y:f32,z:f32)->Vec3{Vec3::new(x,y,z)}
fn tri(z:f32)->Triangle{
    Triangle::new(v(0.0,0.0,z),v(1.0,0.0,z),v(0.0,1.0,z)).unwrap()
}
fn ray(p:Vec3,d:Vec3)->Ray{Ray::new(p,d,0.001,100.0).unwrap()}
fn near(a:f32,b:f32){assert!((a-b).abs()<0.0001,"{a} != {b}");}

#[test]
fn pluecker_triangle_hit_and_uv() {
    let r=ray(v(0.25,0.25,0.0),v(0.0,0.0,1.0));
    let (t,u,bary_v)=triangle_intersect(&r,&tri(2.0),r.t_max).unwrap();
    near(t,2.0);near(u,0.25);near(bary_v,0.25);
    let opposite=ray(r.origin,v(0.0,0.0,-1.0));
    assert!(triangle_intersect(&opposite,&tri(2.0),opposite.t_max).is_none());
    let from_behind=ray(v(0.25,0.25,5.0),v(0.0,0.0,-1.0));
    assert!(triangle_intersect(&from_behind,&tri(2.0),from_behind.t_max).is_some());
}

#[test]
fn closest_hit_and_distance_clipping() {
    let tree=Bvh::build(vec![tri(5.0),tri(2.0),tri(9.0)]);
    let r=ray(v(0.2,0.3,0.0),v(0.0,0.0,1.0));
    let hit=tree.closest_hit(&r).unwrap();
    assert_eq!(hit.triangle_index,1);
    near(hit.t,2.0);
    near(hit.point(&r).z,2.0);
    assert!(tree.occluded(&r));
    let short=Ray::new(r.origin,r.direction,0.001,1.99).unwrap();
    assert!(tree.closest_hit(&short).is_none());
    assert!(!tree.occluded(&short));
    let boundary=Ray::new(r.origin,r.direction,0.001,2.0).unwrap();
    assert!(tree.closest_hit(&boundary).is_some());
}

#[test]
fn aabb_handles_parallel_negative_and_inside_rays() {
    let box3=Aabb{min:v(-1.0,-2.0,3.0),max:v(2.0,3.0,5.0)};
    assert!(box3.entry_distance(&ray(v(0.0,0.0,0.0),v(0.0,0.0,1.0)),100.0).is_some());
    assert!(box3.entry_distance(&ray(v(4.0,0.0,0.0),v(0.0,0.0,1.0)),100.0).is_none());
    assert!(box3.entry_distance(&ray(v(0.0,0.0,6.0),v(0.0,0.0,-1.0)),100.0).is_some());
    assert!(box3.entry_distance(&ray(v(0.0,0.0,4.0),v(0.0,1.0,0.0)),100.0).is_some());
    assert!(Aabb::empty().entry_distance(&ray(v(0.0,0.0,0.0),v(1.0,0.0,0.0)),100.0).is_none());
}

#[test]
fn splits_keep_primitive_indices() {
    let triangles:Vec<_>=(0..40).map(|i|{
        let x=i as f32*2.0;
        Triangle::new(v(x,0.0,3.0),v(x+1.0,0.0,3.0),v(x,1.0,3.0)).unwrap()
    }).collect();
    let tree=Bvh::build(triangles);
    assert_eq!(tree.stats().triangles,40);
    assert!(tree.stats().leaves>1);
    assert!(tree.stats().maximum_depth>1);
    assert!(tree.stats().nodes>1);
    let r=ray(v(16.25,0.25,0.0),v(0.0,0.0,1.0));
    assert_eq!(tree.closest_hit(&r).unwrap().triangle_index,8);
    assert!(MAX_LEAF_TRIANGLES>=1);
}

#[test]
fn rejects_invalid_geometry_and_rays() {
    assert!(Triangle::new(v(0.0,0.0,0.0),v(1.0,1.0,1.0),v(2.0,2.0,2.0)).is_none());
    assert!(Triangle::new(v(f32::NAN,0.0,0.0),v(1.0,1.0,0.0),v(0.0,1.0,0.0)).is_none());
    assert!(Ray::new(v(0.0,0.0,0.0),Vec3::ZERO,0.0,1.0).is_none());
    assert!(Ray::new(v(0.0,0.0,0.0),v(0.0,0.0,1.0),2.0,1.0).is_none());
    let t=tri(2.0);
    let invalid=Triangle{a:Vec3::ZERO,b:Vec3::ZERO,c:Vec3::ZERO};
    let tree=Bvh::build(vec![invalid,t]);
    assert_eq!(tree.stats().triangles,1);
    assert_eq!(tree.closest_hit(&ray(v(0.2,0.2,0.0),v(0.0,0.0,1.0))).unwrap().triangle_index,1);
    assert!(Bvh::build(vec![]).closest_hit(&ray(v(0.0,0.0,0.0),v(0.0,0.0,1.0))).is_none());
}

#[test]
fn triangle_edge_and_miss() {
    let r=ray(v(0.5,0.5,0.0),v(0.0,0.0,1.0));
    assert!(triangle_intersect(&r,&tri(4.0),100.0).is_some());
    let miss=ray(v(1.0,1.0,0.0),v(0.0,0.0,1.0));
    assert!(triangle_intersect(&miss,&tri(4.0),100.0).is_none());
}

#[test]
fn many_rays_match_brute_force() {
    let mut triangles=Vec::new();
    for z in 0..4{
        for x in 0..8{
            for y in 0..8{
                let px=x as f32-4.0;
                let py=y as f32-4.0;
                triangles.push(Triangle::new(v(px,py,1.0+z as f32),v(px+0.9,py,1.0+z as f32),v(px,py+0.9,1.0+z as f32)).unwrap());
            }
        }
    }
    let tree=Bvh::build(triangles.clone());
    for i in 0..320{
        let x=((i*37)%109) as f32/10.0-5.2;
        let y=((i*61)%103) as f32/10.0-5.0;
        let r=ray(v(x,y,-2.0),v(0.0,0.0,1.0));
        let expected=triangles.iter().filter_map(|t|triangle_intersect(&r,t,r.t_max).map(|h|h.0))
            .min_by(f32::total_cmp);
        let result=tree.closest_hit(&r).map(|h|h.t);
        assert_eq!(expected.is_some(),result.is_some(),"ray {i}");
        if let (Some(a),Some(b))=(expected,result){near(a,b);}
    }
}
