//! CirclesRust CPU path-tracing preview (partial Cycles port).
//! Usage: circlesrust-preview [output.png|output.bmp] [width] [height] [samples]
//! Optional: --output, --width, --height, --samples, --bounces, --exposure
use circlesrust::film::save_image;
use circlesrust::integrator::{render_rgb_denoised, RenderSettings};
use circlesrust::scene::{studio_scene,obj_studio_scene};
use circlesrust::scene::obj::ObjMesh;
use std::error::Error;
use std::time::Instant;

fn usage(){
    println!("CirclesRust CPU path tracer — partial Cycles port\n");
    println!("Usage:");
    println!("  circlesrust-preview [output.png|output.bmp] [width] [height] [samples]");
    println!("  circlesrust-preview --output render.png --width 1280 --height 720 --samples 128");
    println!("Optional: --bounces 5 --exposure 1.2 --seed 17 --denoise 2 --obj model.obj");
    println!("Use --no-denoise to export the raw sample accumulation (0..5 denoise strength).");
    println!("Example: circlesrust-preview teste.png 960 540 64");
    println!("Note: CPU-only; not the complete Blender Cycles renderer or GPU path.");
}

fn main()->Result<(),Box<dyn Error>>{
    let mut s=RenderSettings::default();
    let mut output=String::from("circlesrust_render.png");
    let mut denoise_strength: u8=2;
    let mut obj_path: Option<String>=None;
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
    let scene=if let Some(path)=obj_path {
        println!("Loading OBJ: {path}");
        let content=std::fs::read_to_string(&path)?;
        obj_studio_scene(ObjMesh::parse(&content)?)
    }else{studio_scene()};
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
    println!("Features: smoothed normals, area/environment MIS, glossy emitter reflections, multiple bounces, optional a-trous denoise.");
    println!("Denoise strength: {denoise_strength}; this is not OIDN/OptiX AI denoising.");
    println!("Limitations: CPU only; no Cycles node shaders, OptiX/HIP/Metal, denoiser or complex asset import.");
    Ok(())
}
