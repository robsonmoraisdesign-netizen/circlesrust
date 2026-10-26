use circlesrust::film::{bmp_bytes,png_bytes};
use circlesrust::integrator::{render_rgb, sample_pixel, Material, RectLight, RenderSettings, Scene};
use circlesrust::kernel::geometry::Triangle;
use circlesrust::scene::studio_scene;
use circlesrust::util::math::Vec3;

fn v(x:f32,y:f32,z:f32)->Vec3{Vec3::new(x,y,z)}
fn small()->RenderSettings{
    RenderSettings{width:28,height:18,samples:3,max_bounces:3,exposure:1.1,seed:135,..Default::default()}
}
#[test]
fn reproducible_multiple_samples(){
    let scene=studio_scene();
    let s=small();
    let a=render_rgb(&scene,s).unwrap();
    let b=render_rgb(&scene,s).unwrap();
    assert_eq!(a,b,"render must not depend on thread scheduling");
    assert_eq!(a.len(),(s.width*s.height*3) as usize);
    let distinct=a.chunks_exact(3).collect::<std::collections::HashSet<_>>();
    assert!(distinct.len()>25,"image should have different shaded colors");
}
#[test]
fn bsdf_path_is_finite_and_exposure_validation(){
    let scene=studio_scene();
    let s=small();
    for &(x,y) in &[(0,0),(10,8),(20,12),(27,17)]{
        let c=sample_pixel(&scene,s,x,y).unwrap();
        assert!(c.is_finite());
        assert!(c.x>=0.0 && c.y>=0.0 && c.z>=0.0);
    }
    assert!(sample_pixel(&scene,s,28,0).is_err());
    assert!(RenderSettings{samples:0,..s}.validate().is_err());
    assert!(RenderSettings{max_bounces:0,..s}.validate().is_err());
    assert!(RenderSettings{width:5000,..s}.validate().is_err());
}
#[test]
fn png_bmp_export_headers_and_validation(){
    let pixels=[255,0,0,0,255,0,0,0,255,255,255,255];
    let png=png_bytes(&pixels,2,2).unwrap();
    assert_eq!(&png[..8],&[137,80,78,71,13,10,26,10]);
    assert_eq!(&png[12..16],b"IHDR");
    assert_eq!(&png[16..20],&2u32.to_be_bytes());
    assert_eq!(&png[20..24],&2u32.to_be_bytes());
    assert_eq!(&png[png.len()-8..png.len()-4],&[174,66,96,130]);
    let bmp=bmp_bytes(&pixels,2,2).unwrap();
    assert_eq!(&bmp[..2],b"BM");
    assert_eq!(&bmp[18..22],&2u32.to_le_bytes());
    assert_eq!(&bmp[22..26],&2u32.to_le_bytes());
    assert!(png_bytes(&pixels,0,2).is_err());
    assert!(bmp_bytes(&pixels[..3],2,2).is_err());
}
#[test]
fn scene_rejects_unmatched_material_indices(){
    let tri=Triangle::new(v(0.0,0.0,1.0),v(1.0,0.0,1.0),v(0.0,1.0,1.0)).unwrap();
    let mat=Material::new(v(0.5,0.5,0.5),0.0,1.0);
    let light=RectLight{
        center:v(0.0,0.0,3.0), half_u:v(1.0,0.0,0.0),
        half_v:v(0.0,-1.0,0.0), radiance:v(5.0,5.0,5.0)
    };
    assert!(Scene::new(vec![tri],vec![],vec![mat],light).is_err());
    assert!(Scene::new(vec![tri],vec![9],vec![mat],light).is_err());
    assert!(Scene::new(vec![tri],vec![0],vec![mat],light).is_ok());
}
