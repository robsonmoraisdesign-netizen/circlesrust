// SPDX-License-Identifier: Apache-2.0
//! Reproducible volume indicator. Never a claim of functional Cycles parity.
use std::{env,fs,io,path::Path,process::Command};
const REFERENCE:&str="3eb35f85996baa6c1b2e19bf297b22745c85e321";
fn count(path:&Path,extensions:&[&str])->io::Result<usize>{
    let mut total=0;
    for entry in fs::read_dir(path)?{
        let entry=entry?;let kind=entry.file_type()?;let p=entry.path();
        if kind.is_dir(){total+=count(&p,extensions)?;}
        else if kind.is_file()&&p.extension().and_then(|e|e.to_str()).is_some_and(|e|extensions.contains(&e)){
            total+=String::from_utf8_lossy(&fs::read(p)?).lines().filter(|l|!l.trim().is_empty()).count();
        }
    }
    Ok(total)
}
fn run()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<_>=env::args_os().collect();
    if args.len()!=2{return Err("Usage: cargo run --bin migration-progress -- /path/to/clonecircles-reference".into());}
    let reference=Path::new(&args[1]);
    let result=Command::new("git").args(["-C"]).arg(reference).args(["rev-parse","HEAD"]).output()?;
    if !result.status.success()||String::from_utf8_lossy(&result.stdout).trim()!=REFERENCE{
        return Err("Reference checkout must match the pinned commit".into());
    }
    let status=Command::new("git").arg("-C").arg(reference).args(["status","--porcelain","--","src"]).output()?;
    if !status.status.success()||!status.stdout.is_empty(){return Err("Reference src must be unmodified".into());}
    let root=Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut rust=0;
    for module in ["bvh","film","integrator","kernel","scene","shader","util"]{
        rust+=count(&root.join("src").join(module),&["rs"])?;
    }
    let original=count(&reference.join("src"),&["c","cc","cpp","h","hpp","cu","cuh","metal","mm","cl"])?;
    if original==0{return Err("Empty reference source".into());}
    println!("Reference: {REFERENCE}\nRust core nonblank lines: {rust}\nReference nonblank lines: {original}\nVolume indicator: {:.2}%",100.0*rust as f64/original as f64);
    println!("Not a functional completion percentage. Includes comments; excludes Rust UI, tests, CLI, HTML and build scripts. Languages and implementations do not have equivalent line counts.");
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
