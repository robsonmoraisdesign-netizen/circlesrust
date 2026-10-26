// SPDX-License-Identifier: Apache-2.0
//! Limited Wavefront MTL material translation into native Rust BSDF settings.
//! Cycles shader nodes/Principled remain separate, unported subsystems.
use crate::integrator::Material;
use crate::util::math::Vec3;

#[derive(Clone,Debug)]
pub struct NamedMaterial {
    pub name:String,
    pub material:Material,
}
fn finite_unit(text:&str,position:usize,line:usize)->Result<f32,String>{
    let value=text.split_whitespace().nth(position)
        .ok_or_else(||format!("line {line}: missing MTL value"))?
        .parse::<f32>().map_err(|_|format!("line {line}: invalid MTL value"))?;
    if !value.is_finite(){return Err(format!("line {line}: non-finite MTL value"));}
    Ok(value)
}
pub fn parse_mtl(text:&str)->Result<Vec<NamedMaterial>,String>{
    let mut out=Vec::new();
    let mut current:Option<NamedMaterial>=None;
    for (line_number,raw) in text.lines().enumerate(){
        let line=line_number+1;
        let value=raw.split('#').next().unwrap_or("").trim();
        if value.is_empty(){continue;}
        let (tag,arg)=match value.split_once(char::is_whitespace){
            Some((key,rest))=>(key,rest.trim()),
            None=>(value,""),
        };
        if tag=="newmtl"{
            if arg.is_empty(){return Err(format!("line {line}: empty material name"));}
            if let Some(item)=current.take(){out.push(item);}
            current=Some(NamedMaterial{
                name:arg.to_owned(),
                material:Material::new(Vec3::new(0.72,0.72,0.72),0.0,0.6),
            });
        }else if let Some(item)=current.as_mut(){
            match tag{
                "Kd"=>{
                    let x=finite_unit(arg,0,line)?;
                    let y=finite_unit(arg,1,line)?;
                    let z=finite_unit(arg,2,line)?;
                    item.material.base_color=Vec3::new(x.clamp(0.0,1.0),
                        y.clamp(0.0,1.0),z.clamp(0.0,1.0));
                }
                // Extensions written by common PBR-compatible OBJ exporters.
                "Pm"=>item.material.metallic=finite_unit(arg,0,line)?.clamp(0.0,1.0),
                "Pr"=>item.material.roughness=finite_unit(arg,0,line)?.clamp(0.045,1.0),
                "Ns"=>{
                    // Old Phong exponent converted to approximate GGX roughness.
                    // Explicit Pr, if present after Ns, overrides this value.
                    let ns=finite_unit(arg,0,line)?.max(0.0);
                    item.material.roughness=(2.0/(ns+2.0)).sqrt().clamp(0.045,1.0);
                }
                _=>{} // maps/textures/IOR/opacity not yet translated
            }
        }
    }
    if let Some(item)=current{out.push(item);}
    if out.is_empty(){return Err("MTL has no newmtl materials".into());}
    Ok(out)
}
