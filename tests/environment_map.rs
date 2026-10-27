use circlesrust::shader::ImageTexture;
use circlesrust::util::math::Vec3;
use circlesrust::scene::studio_scene;
use circlesrust::integrator::{render_rgb,RenderSettings};

fn v(x:f32,y:f32,z:f32)->Vec3{Vec3::new(x,y,z)}
fn near(a:f32,b:f32){assert!((a-b).abs()<0.001,"{a} vs {b}");}

#[test]
fn hdr_float_values_above_one_remain_unclamped(){
    let texture=ImageTexture::from_rgb32f(
        2,2,&[8.0,4.0,2.0, 8.0,4.0,2.0, 0.1,0.2,0.3, 0.1,0.2,0.3]
    ).unwrap();
    let top=texture.sample_latlong(v(0.0,0.0,1.0));
    near(top.x,8.0);near(top.y,4.0);
    let bottom=texture.sample_latlong(v(0.0,0.0,-1.0));
    near(bottom.x,0.1);near(bottom.z,0.3);
    assert!(ImageTexture::from_rgb32f(2,2,&[1.0;6]).is_err());
    assert!(ImageTexture::from_rgb32f(1,1,&[f32::NAN,0.0,1.0]).is_err());
    assert!(ImageTexture::from_rgb32f(1,1,&[-0.1,0.0,1.0]).is_err());
}

#[test]
fn panoramic_environment_changes_lighting_and_is_repeatable(){
    let options=RenderSettings{
        width:26,height:18,samples:3,max_bounces:3,exposure:1.0,seed:73
    };
    let mut scene=studio_scene();
    let original=render_rgb(&scene,options).unwrap();
    let map=ImageTexture::from_rgb32f(2,2,&[
        3.5,2.2,1.5, 3.5,2.2,1.5,
        0.8,0.4,0.3, 0.8,0.4,0.3,
    ]).unwrap();
    scene.set_environment_map(map);
    let a=render_rgb(&scene,options).unwrap();
    assert_ne!(original,a,"environment should contribute to actual path-traced pixels");
    assert_eq!(a,render_rgb(&scene,options).unwrap());
    scene.sky_strength=0.0;
    let no_sky=render_rgb(&scene,options).unwrap();
    assert_ne!(a,no_sky,"environment strength should change image lighting");
}

#[test]
fn panoramic_ldr_import_works_without_external_engine(){
    let map=ImageTexture::load_environment("tests/fixtures/checker.ppm").unwrap();
    assert!(map.sample_latlong(v(0.0,0.0,1.0)).is_finite());
}
