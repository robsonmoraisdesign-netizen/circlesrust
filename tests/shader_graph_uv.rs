use circlesrust::integrator::{RenderSettings,render_rgb};
use circlesrust::scene::{mtl::parse_mtl,obj::ObjMesh,obj_studio_scene_with_materials};
use circlesrust::shader::{ColorNode,ImageTexture,ShaderGraph,Uv};
use circlesrust::util::math::Vec3;

fn v(x:f32,y:f32,z:f32)->Vec3{Vec3::new(x,y,z)}
fn close(a:f32,b:f32){assert!((a-b).abs()<0.001,"{a} != {b}");}

#[test]
fn uv_interpolation_and_obj_material_maps(){
    let uv=Uv::interpolate([Uv::new(0.0,0.0),Uv::new(1.0,0.0),Uv::new(0.0,1.0)],0.2,0.3);
    close(uv.u,0.2);close(uv.v,0.3);
    let obj=ObjMesh::parse(include_str!("fixtures/textured.obj")).unwrap();
    assert_eq!(obj.triangles.len(),2);
    assert!(obj.corner_uvs.iter().all(|x|x.is_some()));
    close(obj.corner_uvs[0].unwrap()[0].u,0.0);
    let mat=parse_mtl(include_str!("fixtures/textured.mtl")).unwrap();
    assert_eq!(mat[0].map_kd.as_deref(),Some("checker.ppm"));
}

#[test]
fn image_texture_srgb_bilinear_repeat_and_flip(){
    // First row is top. OBJ v=1 selects the first image row.
    let rgb=[255,0,0, 0,255,0, 0,0,255, 255,255,255];
    let t=ImageTexture::from_rgb8(2,2,&rgb).unwrap();
    let red=t.sample(Uv::new(0.25,0.75));
    close(red.x,1.0);close(red.y,0.0);close(red.z,0.0);
    let blue=t.sample(Uv::new(0.25,0.25));
    close(blue.z,1.0);
    let repeated=t.sample(Uv::new(1.25,-0.25));
    close(repeated.x,red.x);
    let center=t.sample(Uv::new(0.5,0.5));
    close(center.x,0.5);close(center.y,0.5);close(center.z,0.5);
    assert!(ImageTexture::from_rgb8(0,2,&rgb).is_err());
    assert!(ImageTexture::from_rgb8(2,2,&rgb[..6]).is_err());
}

#[test]
fn shader_graphs_validate_connections_and_mix_nodes(){
    assert!(ShaderGraph::new(vec![ColorNode::Multiply{left:0,right:0}],0).is_err());
    let g=ShaderGraph::new(vec![
        ColorNode::Constant(v(1.0,0.0,0.0)),
        ColorNode::Constant(v(0.0,0.0,1.0)),
        ColorNode::Mix{left:0,right:1,factor:0.25},
    ],2).unwrap();
    let out=g.evaluate(Uv::new(0.0,0.0));
    close(out.x,0.75);close(out.z,0.25);
    let checker=ShaderGraph::new(vec![ColorNode::Checker{
        first:v(1.0,1.0,1.0),second:Vec3::ZERO,scale:4.0,
    }],0).unwrap();
    assert!(checker.evaluate(Uv::new(0.1,0.1)).x>0.9);
    assert!(checker.evaluate(Uv::new(0.3,0.1)).x<0.1);
}

#[test]
fn texture_files_change_path_traced_pixels(){
    let mtl=parse_mtl(include_str!("fixtures/textured.mtl")).unwrap();
    let mut scene=obj_studio_scene_with_materials(
        ObjMesh::parse(include_str!("fixtures/textured.obj")).unwrap(),&mtl);
    let options=RenderSettings{
        width:42,height:30,samples:5,max_bounces:3,exposure:1.0,seed:141
    };
    let baseline=render_rgb(&scene,options).unwrap();
    let texture=ImageTexture::load("tests/fixtures/checker.ppm").unwrap();
    let graph=ShaderGraph::diffuse_map(mtl[0].material.base_color,texture);
    scene.set_shader_graphs(vec![None,None,Some(graph)]).unwrap();
    let textured=render_rgb(&scene,options).unwrap();
    assert_eq!(textured.len(),baseline.len());
    assert_ne!(textured,baseline,"sampled UV texture must affect ray traced pixels");
    let repeated=render_rgb(&scene,options).unwrap();
    assert_eq!(repeated,textured,"same texture, settings and seed must be deterministic");
    assert!(scene.set_shader_graphs(vec![None]).is_err());
}
