// SPDX-License-Identifier: Apache-2.0
//! One Rust scene loader shared by CLI and native desktop interface.
//! MTL textures resolve relative to the MTL file, never an engine installation.
use std::path::{Path,PathBuf};
use crate::integrator::Scene;
use crate::scene::{studio_scene,obj_studio_scene_with_materials,mtl::{parse_mtl,NamedMaterial},obj::ObjMesh};
use crate::shader::{ImageTexture,ShaderGraph,TextureColorSpace};
use crate::shader::surface::SurfaceMaps;

#[derive(Clone,Debug,Default)]
pub struct SceneLoadOptions {
    pub obj:Option<PathBuf>,
    pub mtl:Option<PathBuf>,
    pub environment:Option<PathBuf>,
    pub environment_strength:f32,
}
impl SceneLoadOptions {
    pub fn studio()->Self{
        Self{environment_strength:1.0,..Self::default()}
    }
}
pub struct LoadedScene {
    pub scene:Scene,
    pub material_names:Vec<String>,
    pub source_label:String,
}
fn load_mtl(path:&Path)->Result<Vec<NamedMaterial>,String>{
    let text=std::fs::read_to_string(path)
        .map_err(|e|format!("Unable to read MTL {}: {e}",path.display()))?;
    let mut materials=parse_mtl(&text)?;
    let folder=path.parent().unwrap_or(Path::new("."));
    for material in &mut materials{
        for path in [
            &mut material.map_kd,&mut material.map_pr,
            &mut material.map_pm,&mut material.map_normal
        ].into_iter().flatten(){
            *path=folder.join(&*path).to_string_lossy().into_owned();
        }
    }
    Ok(materials)
}
pub fn load_scene(options:&SceneLoadOptions)->Result<LoadedScene,String>{
    let strength=options.environment_strength;
    if !strength.is_finite() || !(0.0..=1000.0).contains(&strength){
        return Err("Environment strength must be in 0..=1000".into());
    }
    if options.obj.is_none() && options.mtl.is_some(){
        return Err("An MTL library needs an OBJ mesh".into());
    }
    let mut loaded=if let Some(path)=&options.obj {
        let text=std::fs::read_to_string(path)
            .map_err(|e|format!("Unable to read OBJ {}: {e}",path.display()))?;
        let mesh=ObjMesh::parse(&text)?;
        let mut named=Vec::new();
        if let Some(mtl)=&options.mtl{
            named.extend(load_mtl(mtl)?);
        }else{
            let folder=path.parent().unwrap_or(Path::new("."));
            for library in &mesh.material_libraries{
                let asset=folder.join(library);
                if asset.exists(){named.extend(load_mtl(&asset)?);}
            }
        }
        let mut graphs=vec![None,None];
        let mut maps=vec![SurfaceMaps::default(),SurfaceMaps::default()];
        for material in &named{
            graphs.push(material.map_kd.as_ref()
                .map(|path|ImageTexture::load(path)
                    .map(|image|ShaderGraph::diffuse_map(material.material.base_color,image)))
                .transpose()?);
            let physical=|p:&Option<String>|p.as_ref().map(|path|{
                ImageTexture::load_with_space(path,TextureColorSpace::NonColor)
            }).transpose();
            maps.push(SurfaceMaps{
                roughness:physical(&material.map_pr)?,
                metallic:physical(&material.map_pm)?,
                normal:physical(&material.map_normal)?
            });
        }
        let mut scene=obj_studio_scene_with_materials(mesh,&named);
        scene.set_shader_graphs(graphs).map_err(str::to_owned)?;
        scene.set_surface_maps(maps).map_err(str::to_owned)?;
        let mut names=vec!["Piso".to_owned(),"Material padrão".to_owned()];
        names.extend(named.into_iter().map(|m|m.name));
        LoadedScene{
            scene,material_names:names,
            source_label:path.file_name().and_then(|s|s.to_str())
                .unwrap_or("OBJ").to_owned(),
        }
    }else{
        LoadedScene {
            scene:studio_scene(),
            material_names:["Piso","Fundo","Cerâmica","Metal Azul","Metal Champagne"]
                .into_iter().map(str::to_owned).collect(),
            source_label:"Cena de estúdio".into(),
        }
    };
    loaded.scene.sky_strength=strength;
    if let Some(path)=&options.environment{
        let environment=ImageTexture::load_environment(path)?;
        loaded.scene.set_environment_map(environment);
    }
    Ok(loaded)
}
