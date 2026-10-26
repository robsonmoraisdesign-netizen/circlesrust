use circlesrust::film::denoise::{denoise_rgb, PixelGuide};
use circlesrust::integrator::{render_rgb, render_rgb_denoised, sample_pixel, Material, RectLight, RenderSettings, Scene};
use circlesrust::scene::studio_scene;
use circlesrust::util::math::Vec3;

fn v(x:f32,y:f32,z:f32)->Vec3 {Vec3::new(x,y,z)}
fn options()->RenderSettings {
    RenderSettings{width:32,height:24,samples:4,max_bounces:4,exposure:1.0,seed:74}
}

#[test]
fn studio_sphere_has_interpolated_normals(){
    let scene=studio_scene();
    let sphere=scene.corner_normals.iter().position(|n|n.is_some()).expect("smooth sphere normals");
    let tri=scene.bvh.triangle(sphere).unwrap();
    let geometric=tri.normal();
    let smooth=scene.shading_normal(sphere,0.2,0.3);
    assert!(smooth.is_finite());
    assert!(smooth.dot(geometric)>0.0);
    assert!((smooth-geometric).length_squared()>1e-7,"barycentric normal differs from face normal");
    assert!(scene.corner_normals.iter().any(|x|x.is_none()),"cube and floor faces remain flat");
}

#[test]
fn corner_normals_validate_shape(){
    let mut scene=studio_scene();
    assert!(scene.set_corner_normals(vec![None]).is_err());
    let mut normals=vec![None;scene.triangle_materials.len()];
    normals[0]=Some([Vec3::ZERO,v(0.0,0.0,1.0),v(0.0,0.0,1.0)]);
    assert!(scene.set_corner_normals(normals).is_err());
}

#[test]
fn guided_denoise_preserves_surface_edges(){
    let mut data=vec![0u8;7*5*3];
    let mut guides=Vec::new();
    for _y in 0..5{
        for x in 0..7{
            let idx=(guides.len())*3;
            let side=if x<3{0}else{1};
            for color in &mut data[idx..idx+3]{*color=if side==0{12}else{238};}
            guides.push(PixelGuide{normal:v(0.0,0.0,1.0),depth:4.0,surface:Some(side)});
        }
    }
    let denoised=denoise_rgb(&data,7,5,&guides,3).unwrap();
    assert_eq!(denoised,data,"different surface IDs must not bleed");
    assert_eq!(denoise_rgb(&data,7,5,&guides,0).unwrap(),data);
    assert!(denoise_rgb(&data,7,5,&guides[..5],3).is_err());
    assert!(denoise_rgb(&data,7,5,&guides,6).is_err());
}

#[test]
fn constant_sky_denoises_without_invalid_infinities(){
    let guides=vec![PixelGuide::SKY;9];
    let color=vec![100u8;9*3];
    let output=denoise_rgb(&color,3,3,&guides,3).unwrap();
    assert_eq!(output,color);
}

#[test]
fn path_render_with_denoiser_is_deterministic(){
    let scene=studio_scene();
    let settings=options();
    let a=render_rgb_denoised(&scene,settings,2).unwrap();
    let b=render_rgb_denoised(&scene,settings,2).unwrap();
    assert_eq!(a,b);
    let raw=render_rgb(&scene,settings).unwrap();
    assert_eq!(render_rgb_denoised(&scene,settings,0).unwrap(),raw);
    assert_eq!(a.len(),(settings.width*settings.height*3) as usize);
}

#[test]
fn emissive_rectangle_is_visible_to_primary_rays(){
    let material=Material::new(v(0.5,0.5,0.5),0.0,1.0);
    // Cross(+X,+Z) points toward -Y, i.e. towards the studio camera.
    let emitter=RectLight{
        center:v(0.0,0.15,0.96),
        half_u:v(1.0,0.0,0.0),
        half_v:v(0.0,0.0,1.0),
        radiance:v(3.0,2.0,1.0),
    };
    let mut scene=Scene::new(vec![],vec![],vec![material],emitter).unwrap();
    scene.sky_strength=0.0;
    let settings=RenderSettings{width:16,height:16,samples:16,max_bounces:3,exposure:1.0,seed:5};
    let sample=sample_pixel(&scene,settings,8,8).unwrap();
    assert!(sample.x>0.0 && sample.y>0.0 && sample.z>0.0,
            "area light must be visible directly rather than only through shadow rays");
}
