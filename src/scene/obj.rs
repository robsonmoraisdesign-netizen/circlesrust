// SPDX-License-Identifier: Apache-2.0
//! Wavefront OBJ static-triangle mesh import with material assignments.
//! Supports vertex positions, UVs, optional vertex normals, triangles and polygon fans.
//! This does not implement Cycles' complete mesh/attribute/scene import.
use crate::kernel::geometry::Triangle;
use crate::util::math::Vec3;
use crate::shader::Uv;

#[derive(Debug)]
pub struct ObjMesh {
    pub triangles: Vec<Triangle>,
    pub corner_normals: Vec<Option<[Vec3;3]>>,
    pub corner_uvs: Vec<Option<[Uv;3]>>,
    /// Material name for each valid triangulated face, if assigned by usemtl.
    pub face_materials: Vec<Option<String>>,
    /// Per-triangle Wavefront `o` or `g` name, preserving source object groups.
    pub face_groups: Vec<String>,
    pub group_names: Vec<String>,
    /// mtllib references, for loading from the OBJ directory.
    pub material_libraries: Vec<String>,
}

fn vertex_index(input:&str,size:usize,line:usize)->Result<usize,String>{
    let raw=input.parse::<i64>().map_err(|_|format!("line {line}: malformed OBJ index"))?;
    let index=if raw>0 {raw-1} else if raw<0 {size as i64+raw} else {-1};
    if index<0 || index>=size as i64 {
        Err(format!("line {line}: OBJ vertex/normal index out of range"))
    } else {Ok(index as usize)}
}
fn vector3<'a>(mut parts:impl Iterator<Item=&'a str>,line:usize)->Result<Vec3,String>{
    let x=parts.next().ok_or_else(||format!("line {line}: missing X coordinate"))?
        .parse::<f32>().map_err(|_|format!("line {line}: invalid coordinate"))?;
    let y=parts.next().ok_or_else(||format!("line {line}: missing Y coordinate"))?
        .parse::<f32>().map_err(|_|format!("line {line}: invalid coordinate"))?;
    let z=parts.next().ok_or_else(||format!("line {line}: missing Z coordinate"))?
        .parse::<f32>().map_err(|_|format!("line {line}: invalid coordinate"))?;
    let value=Vec3::new(x,y,z);
    if !value.is_finite(){return Err(format!("line {line}: non-finite vector"));}
    Ok(value)
}
impl ObjMesh {
    pub fn parse(src:&str)->Result<Self,String>{
        let mut vertices=Vec::new();
        let mut normals=Vec::new();
        let mut texcoords=Vec::new();
        let mut triangles=Vec::new();
        let mut corner_normals=Vec::new();
        let mut corner_uvs=Vec::new();
        let mut face_materials=Vec::new();
        let mut face_groups=Vec::new();
        let mut current_group=String::from("Objeto");
        let mut group_names=vec![current_group.clone()];
        let mut material_libraries=Vec::new();
        let mut current_material:Option<String>=None;
        for (i,line) in src.lines().enumerate(){
            let number=i+1;
            let mut parts=line.split_whitespace();
            let Some(tag)=parts.next() else{continue;};
            match tag {
                "v"=>vertices.push(vector3(parts,number)?),
                "vt"=>{
                    let u=parts.next().ok_or_else(||format!("line {number}: missing UV U"))?
                        .parse::<f32>().map_err(|_|format!("line {number}: invalid UV U"))?;
                    let v=parts.next().ok_or_else(||format!("line {number}: missing UV V"))?
                        .parse::<f32>().map_err(|_|format!("line {number}: invalid UV V"))?;
                    let uv=Uv::new(u,v);
                    if !uv.valid(){return Err(format!("line {number}: invalid UV"));}
                    texcoords.push(uv);
                },
                "vn"=>normals.push(vector3(parts,number)?.normalized()
                    .ok_or_else(||format!("line {number}: zero-length normal"))?),
                "mtllib"=>{
                    // Multiple MTL library filenames are supported for simple paths.
                    material_libraries.extend(parts.filter(|p| !p.starts_with('#')).map(str::to_owned));
                },
                "o"|"g"=>{
                    let name=parts.take_while(|p|!p.starts_with('#')).collect::<Vec<_>>().join(" ");
                    current_group=if name.is_empty(){"Objeto".into()}else{name};
                    if !group_names.contains(&current_group){
                        group_names.push(current_group.clone());
                    }
                },
                "usemtl"=>{
                    let name=parts.collect::<Vec<_>>().join(" ");
                    current_material=(!name.is_empty()).then_some(name);
                },
                "f"=>{
                    let mut indices=Vec::new();
                    for token in parts.take_while(|t| !t.starts_with('#')){
                        if indices.len()>=256 {return Err(format!("line {number}: polygon too large"));}
                        let fields:Vec<_>=token.split('/').collect();
                        if fields.len()>3 {return Err(format!("line {number}: invalid face token"));}
                        let position=vertex_index(fields[0],vertices.len(),number)?;
                        let uv=fields.get(1).filter(|s|!s.is_empty()).map(|s|{
                            vertex_index(s,texcoords.len(),number)
                        }).transpose()?;
                        let normal=fields.get(2).filter(|s|!s.is_empty()).map(|s|{
                            vertex_index(s,normals.len(),number)
                        }).transpose()?;
                        indices.push((position,uv,normal));
                    }
                    if indices.len()<3{return Err(format!("line {number}: face needs 3 vertices"));}
                    // Fan triangulation: reliable for convex polygon faces.
                    for j in 1..indices.len()-1{
                        let face=[indices[0],indices[j],indices[j+1]];
                        let xyz=face.map(|(idx,_,_)|vertices[idx]);
                        let Some(tri)=Triangle::new(xyz[0],xyz[1],xyz[2]) else{continue;};
                        let smooth=match (face[0].2,face[1].2,face[2].2){
                            (Some(a),Some(b),Some(c))=>Some([normals[a],normals[b],normals[c]]),
                            _=>None,
                        };
                        let tex=match (face[0].1,face[1].1,face[2].1){
                            (Some(a),Some(b),Some(c))=>Some([texcoords[a],texcoords[b],texcoords[c]]),
                            _=>None,
                        };
                        triangles.push(tri);
                        corner_normals.push(smooth);
                        corner_uvs.push(tex);
                        face_materials.push(current_material.clone());
                        face_groups.push(current_group.clone());
                    }
                },
                _=>{}, // optional vt, mtllib, usemtl, groups etc. skipped for this milestone
            }
        }
        // Keep only groups that actually own geometry, preserving first appearance.
        group_names.clear();
        for group in &face_groups{
            if !group_names.contains(group){group_names.push(group.clone());}
        }
        if triangles.is_empty(){return Err("OBJ has no valid triangles".into());}
        Ok(Self{triangles,corner_normals,corner_uvs,face_materials,face_groups,group_names,material_libraries})
    }

    /// Uniformly fit imported geometry near the studio camera's focal point.
    /// Keeps normals unchanged because only uniform positive scaling is used.
    pub fn fit_to_studio(&mut self){
        let mut min=Vec3::new(f32::INFINITY,f32::INFINITY,f32::INFINITY);
        let mut max=Vec3::new(f32::NEG_INFINITY,f32::NEG_INFINITY,f32::NEG_INFINITY);
        for tri in &self.triangles{
            for p in [tri.a,tri.b,tri.c]{
                min=min.min(p);max=max.max(p);
            }
        }
        let center=(min+max)*0.5;
        let extent=max-min;
        let scale=2.4/(extent.x.max(extent.y).max(extent.z).max(0.0001));
        for tri in &mut self.triangles{
            for p in [&mut tri.a,&mut tri.b,&mut tri.c] {
                let original=*p;
                *p=(original-center)*scale+Vec3::new(0.0,0.0,(center.z-min.z)*scale);
            }
        }
    }
}
