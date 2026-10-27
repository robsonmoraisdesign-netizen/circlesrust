use circlesrust::integrator::{Material,RenderSettings,render_rgb};
use circlesrust::scene::{obj::ObjMesh,mtl::parse_mtl,obj_studio_scene_with_materials};
use circlesrust::shader::{ImageTexture,TextureColorSpace,Uv};
use circlesrust::shader::surface::SurfaceMaps;
use circlesrust::kernel::geometry::Triangle;
use circlesrust::util::math::Vec3;

fn v(x:f32,y:f32,z:f32)->Vec3{Vec3::new(x,y,z)}
fn options()->RenderSettings{
    RenderSettings{width:48,height:32,samples:6,max_bounces:4,exposure:1.0,seed:2048}
}
fn mat()->Material{Material::new(v(0.75,0.65,0.5),0.0,0.72)}
fn raw(value:&[u8])->ImageTexture{
    ImageTexture::from_rgb8_with_space(1,1,value,TextureColorSpace::NonColor).unwrap()
}

#[test]
fn numeric_noncolor_maps_keep_byte_values_linear(){
    let rgb=[128,128,128];
    let raw=ImageTexture::from_rgb8_with_space(1,1,&rgb,TextureColorSpace::NonColor).unwrap();
    let srgb=ImageTexture::from_rgb8(1,1,&rgb).unwrap();
    let uv=Uv::new(0.5,0.5);
    let noncolor=raw.sample(uv).x;
    assert!((noncolor-128.0/255.0).abs()<1e-6);
    assert!(srgb.sample(uv).x<noncolor*0.5,"roughness must NOT use sRGB transfer");
}

#[test]
fn scalar_maps_override_metallic_roughness_and_use_uv(){
    let map=ImageTexture::from_rgb8_with_space(2,1,&[0,0,0,255,255,255],TextureColorSpace::NonColor).unwrap();
    let maps=SurfaceMaps{roughness:Some(map.clone()),metallic:Some(map),normal:None};
    let left=maps.resolved_material(mat(),Uv::new(0.25,0.5));
    assert!(left.metallic<0.01);
    assert!((left.roughness-0.045).abs()<1e-6);
    let right=maps.resolved_material(mat(),Uv::new(0.75,0.5));
    assert!(right.metallic>0.99);
    assert!(right.roughness>0.99);
}

#[test]
fn tangent_normals_respect_uv_handedness_and_handle_degenerate_uv(){
    let tri=Triangle::new(v(0.0,0.0,0.0),v(1.0,0.0,0.0),v(0.0,1.0,0.0)).unwrap();
    let corners=[Uv::new(0.0,0.0),Uv::new(1.0,0.0),Uv::new(0.0,1.0)];
    let maps=SurfaceMaps{normal:Some(raw(&[255,128,255])),..Default::default()};
    let mapped=maps.mapped_normal(&tri,corners,Uv::new(0.5,0.5),v(0.0,0.0,1.0)).unwrap();
    assert!(mapped.x>0.6 && mapped.z>0.6,"+X tangent normal must tilt toward +X");
    let mirrored=[Uv::new(0.0,0.0),Uv::new(0.0,1.0),Uv::new(1.0,0.0)];
    let flipped=maps.mapped_normal(&tri,mirrored,Uv::new(0.5,0.5),v(0.0,0.0,1.0)).unwrap();
    assert!(flipped.y>0.6,"mirrored UV changes the tangent direction");
    let invalid=[Uv::new(0.0,0.0);3];
    assert!(maps.mapped_normal(&tri,invalid,Uv::new(0.5,0.5),v(0.0,0.0,1.0)).is_none());
}

#[test]
fn mtl_parses_pbr_paths_without_treating_height_as_normal(){
    let text=include_str!("fixtures/pbr_textures.mtl");
    let mats=parse_mtl(text).unwrap();
    assert_eq!(mats[0].map_pm.as_deref(),Some("metallic.ppm"));
    assert_eq!(mats[0].map_pr.as_deref(),Some("roughness.ppm"));
    assert_eq!(mats[0].map_normal.as_deref(),Some("angled-normal.ppm"));
    let height=parse_mtl("newmtl height\nmap_Bump height.png\n").unwrap();
    assert!(height[0].map_normal.is_none());
    assert!(parse_mtl("newmtl test\nmap_Pr -s 2 2 2 image.png").is_err());
}

#[test]
fn maps_reach_path_tracing_and_change_result(){
    let mesh=ObjMesh::parse(include_str!("fixtures/textured.obj")).unwrap();
    let mats=parse_mtl(include_str!("fixtures/textured.mtl")).unwrap();
    let mut scene=obj_studio_scene_with_materials(mesh,&mats);
    let baseline=render_rgb(&scene,options()).unwrap();
    let metallic=ImageTexture::load_with_space(
        "tests/fixtures/metallic.ppm",TextureColorSpace::NonColor
    ).unwrap();
    let roughness=ImageTexture::load_with_space(
        "tests/fixtures/roughness.ppm",TextureColorSpace::NonColor
    ).unwrap();
    scene.set_surface_maps(vec![
        SurfaceMaps::default(),SurfaceMaps::default(),
        SurfaceMaps{roughness:Some(roughness),metallic:Some(metallic),normal:None}
    ]).unwrap();
    let modified=render_rgb(&scene,options()).unwrap();
    assert_ne!(baseline,modified,"non-color maps must change path-traced pixels");
    assert_eq!(modified,render_rgb(&scene,options()).unwrap());
    assert!(scene.set_surface_maps(vec![]).is_err());
}
