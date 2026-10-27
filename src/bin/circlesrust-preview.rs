//! CirclesRust CPU path-tracing preview (partial Cycles port).
//! Usage: circlesrust-preview [output.png|output.bmp] [width] [height] [samples]
//! Optional: --output, --width, --height, --samples, --bounces, --exposure
use circlesrust::film::save_image;
use circlesrust::integrator::{render_rgb_denoised, RenderSettings};
use circlesrust::scene::{studio_scene,obj_studio_scene_with_materials};
use circlesrust::scene::mtl::{parse_mtl,NamedMaterial};
use circlesrust::shader::{ImageTexture,ShaderGraph,TextureColorSpace};
use circlesrust::shader::surface::SurfaceMaps;
use circlesrust::scene::obj::ObjMesh;
use std::error::Error;
use std::time::Instant;
use std::path::Path;

fn usage(){
    println!("CirclesRust CPU path tracer — partial Cycles port\n");
    println!("Usage:");
    println!("  circlesrust-preview [output.png|output.bmp] [width] [height] [samples]");
    println!("  circlesrust-preview --output render.png --width 1280 --height 720 --samples 128");
    println!("Optional: --bounces 5 --exposure 1.2 --seed 17 --denoise 2 --obj model.obj --mtl model.mtl");
    println!("Optional environment: --environment studio.hdr --env-strength 1.0 (also PNG/JPEG).");
    println!("Use --no-denoise to export the raw sample accumulation (0..5 denoise strength).");
    println!("Example: circlesrust-preview teste.png 960 540 64");
    println!("Note: CPU-only; not the complete Blender Cycles renderer or GPU path.");
}

/// Resolve texture files relative to their own MTL, not the working directory.
fn mtl_from_path(path:&Path)->Result<Vec<NamedMaterial>,Box<dyn Error>>{
    let data=std::fs::read_to_string(path)?;
    let mut named=parse_mtl(&data)?;
    let base=path.parent().unwrap_or(Path::new("."));
    for m in &mut named{
        for source in [
            &mut m.map_kd, &mut m.map_pr,
            &mut m.map_pm, &mut m.map_normal,
        ].into_iter().flatten(){
            let joined=base.join(&*source);
            *source=joined.to_string_lossy().into_owned();
        }
    }
    Ok(named)
}

fn main()->Result<(),Box<dyn Error>>{
    let mut s=RenderSettings::default();
    let mut output=String::from("circlesrust_render.png");
    let mut denoise_strength: u8=2;
    let mut obj_path: Option<String>=None;
    let mut mtl_path: Option<String>=None;
    let mut env_path: Option<String>=None;
    let mut env_strength:Option<f32>=None;
    let mut positional=Vec::new();
    let args:Vec<String>=std::env::args().skip(1).collect();
    let mut i=0usize;
    while i<args.len(){
        let arg=args[i].as_str();
        if arg=="--help" || arg=="-h" {usage();return Ok(());}
        if arg=="--no-denoise" {denoise_strength=0;i+=1;continue;}
        if arg.starts_with("--"){
            i+=1;
            let raw=args.get(i).ok_or("Option requires a value")?;
            match arg{
                "--output"=>output=raw.clone(),
                "--width"=>s.width=raw.parse()?,
                "--height"=>s.height=raw.parse()?,
                "--samples"=>s.samples=raw.parse()?,
                "--bounces"=>s.max_bounces=raw.parse()?,
                "--exposure"=>s.exposure=raw.parse()?,
                "--seed"=>s.seed=raw.parse()?,
                "--denoise"=>denoise_strength=raw.parse()?,
                "--obj"=>obj_path=Some(raw.clone()),
                "--mtl"=>mtl_path=Some(raw.clone()),
                "--environment"=>env_path=Some(raw.clone()),
                "--env-strength"=>env_strength=Some(raw.parse()?),
                _=>return Err(format!("Unknown option: {arg}").into()),
            }
        }else{
            positional.push(arg.to_owned());
        }
        i+=1;
    }
    if positional.len()>4{return Err("Too many positional arguments".into());}
    if let Some(name)=positional.first(){output=name.to_owned();}
    if let Some(w)=positional.get(1){s.width=w.parse()?;}
    if let Some(h)=positional.get(2){s.height=h.parse()?;}
    if let Some(samples)=positional.get(3){s.samples=samples.parse()?;}
    s.validate()?;

    let start=Instant::now();
    if obj_path.is_none() && mtl_path.is_some() {
        return Err("--mtl requires --obj".into());
    }
    let mut scene=if let Some(path)=obj_path {
        println!("Loading OBJ: {path}");
        let content=std::fs::read_to_string(&path)?;
        let mesh=ObjMesh::parse(&content)?;
        let mut named=Vec::new();
        if let Some(mtl)=mtl_path{
            named.extend(mtl_from_path(Path::new(&mtl))?);
        }else{
            // Follow OBJ mtllib references relative to the model's directory.
            // Missing file is a warning so untextured OBJ still renders.
            let parent=std::path::Path::new(&path).parent().unwrap_or(std::path::Path::new("."));
            for library in &mesh.material_libraries{
                let asset=parent.join(library);
                if asset.exists(){
                    named.extend(mtl_from_path(&asset)?);
                }else{
                    eprintln!("Warning: missing material library {}",asset.display());
                }
            }
        }
        println!("Loaded {} OBJ faces and {} named materials",mesh.triangles.len(),named.len());
        // Slot 0: ground; slot 1: neutral fallback; named materials start at 2.
        let mut graphs=vec![None,None];
        let mut surface_maps=vec![SurfaceMaps::default(),SurfaceMaps::default()];
        for item in &named{
            let graph=match &item.map_kd{
                Some(path)=>{
                    println!("Decoding diffuse texture: {path}");
                    let image=ImageTexture::load(path)?;
                    Some(ShaderGraph::diffuse_map(item.material.base_color,image))
                }
                None=>None,
            };
            graphs.push(graph);
            let load_map=|path:&Option<String>|->Result<Option<ImageTexture>,String>{
                path.as_ref().map(|file|ImageTexture::load_with_space(
                    file,TextureColorSpace::NonColor
                )).transpose()
            };
            surface_maps.push(SurfaceMaps{
                roughness:load_map(&item.map_pr)?,
                metallic:load_map(&item.map_pm)?,
                normal:load_map(&item.map_normal)?,
            });
        }
        let mut scene=obj_studio_scene_with_materials(mesh,&named);
        scene.set_shader_graphs(graphs)?;
        scene.set_surface_maps(surface_maps)?;
        scene
    }else{studio_scene()};
    if let Some(strength)=env_strength{
        if !strength.is_finite() || strength<0.0 || strength>1000.0{
            return Err("--env-strength must be finite in 0..=1000".into());
        }
        scene.sky_strength=strength;
    }
    if let Some(file)=env_path{
        let map=ImageTexture::load_environment(&file)?;
        scene.set_environment_map(map);
        println!("Environment loaded: {file}");
    }
    let stats=scene.bvh.stats();
    println!("CirclesRust CPU path tracing: {}x{} / {} samples / {} bounces",
        s.width,s.height,s.samples,s.max_bounces);
    println!("Scene geometry: {} triangles, {} BVH nodes ({} leaves)",
        stats.triangles,stats.nodes,stats.leaves);
    println!("Tracing primary, shadow and indirect paths...");
    if denoise_strength>5{return Err("Denoise strength must be 0..5".into());}
    let rgb=render_rgb_denoised(&scene,s,denoise_strength)?;
    save_image(&output,&rgb,s.width,s.height)?;
    println!("Saved {output} in {:.2}s",start.elapsed().as_secs_f64());
    println!("Features: UV PBR maps, LDR/HDR panoramic environment, area/environment MIS, indirect bounces, denoise.");
    println!("Denoise strength: {denoise_strength}; this is not OIDN/OptiX AI denoising.");
    println!("Limitations: CPU-only; approximate environment sampling, no Cycles Principled/OSL, GPU or OIDN.");
    Ok(())
}
