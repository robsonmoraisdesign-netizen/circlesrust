//! BVH CPU diagnostic BMP renderer. Not Cycles PBR or the Cycles path tracer.
use circlesrust::bvh::Bvh;
use circlesrust::kernel::geometry::{Ray,Triangle};
use circlesrust::util::math::Vec3;
use std::{error::Error,fs};

fn v(x:f32,y:f32,z:f32)->Vec3{Vec3::new(x,y,z)}
fn quad(t:&mut Vec<Triangle>,a:Vec3,b:Vec3,c:Vec3,d:Vec3){
    t.push(Triangle::new(a,b,c).expect("valid triangle"));
    t.push(Triangle::new(a,c,d).expect("valid triangle"));
}
fn cube(t:&mut Vec<Triangle>,center:Vec3,r:f32){
    let pts=[
        v(-1.0,-1.0,-1.0),v(1.0,-1.0,-1.0),
        v(1.0,1.0,-1.0),v(-1.0,1.0,-1.0),
        v(-1.0,-1.0,1.0),v(1.0,-1.0,1.0),
        v(1.0,1.0,1.0),v(-1.0,1.0,1.0),
    ].map(|p|center+p*r);
    for [a,b,c,d] in [[0,3,2,1],[4,5,6,7],[0,1,5,4],
                       [1,2,6,5],[2,3,7,6],[3,0,4,7]] {
        quad(t,pts[a],pts[b],pts[c],pts[d]);
    }
}
fn scene()->Bvh{
    let mut t=Vec::new();
    quad(&mut t,v(-6.0,-6.0,0.0),v(6.0,-6.0,0.0),
        v(6.0,6.0,0.0),v(-6.0,6.0,0.0));
    cube(&mut t,v(-0.65,0.2,0.84),0.84);
    cube(&mut t,v(1.15,0.8,0.50),0.50);
    Bvh::build(t)
}
fn channel(value:f32)->u8{
    (value.clamp(0.0,1.0).powf(1.0/2.2)*255.0+0.5) as u8
}
fn shade(bvh:&Bvh,ray:&Ray,y:f32)->[u8;3]{
    let Some(hit)=bvh.closest_hit(ray) else {
        let t=y.clamp(0.0,1.0);
        return [channel(0.05+t*0.06),channel(0.08+t*0.08),channel(0.12+t*0.11)];
    };
    let n=bvh.triangle(hit.triangle_index).expect("triangle ID").normal();
    let base=match hit.triangle_index {
        0..=1=>v(0.51,0.57,0.65),
        2..=13=>v(0.90,0.48,0.20),
        _=>v(0.20,0.52,0.84),
    };
    let light=v(-0.55,-0.35,1.0).normalized().expect("light direction");
    let shadow=Ray::new(hit.point(ray)+n*0.002,light,0.002,100.0).expect("shadow ray");
    let blocked=bvh.occluded(&shadow);
    let diffuse=n.dot(light).max(0.0)*if blocked{0.14}else{0.8};
    let intensity=0.23+diffuse;
    [channel(base.x*intensity),channel(base.y*intensity),channel(base.z*intensity)]
}
fn bmp(bvh:&Bvh,width:u32,height:u32)->Vec<u8>{
    let stride=(width as usize*3+3)&!3;
    let size=54+stride*height as usize;
    let mut bytes=vec![0u8;size];
    bytes[0..2].copy_from_slice(b"BM");
    bytes[2..6].copy_from_slice(&(size as u32).to_le_bytes());
    bytes[10..14].copy_from_slice(&54u32.to_le_bytes());
    bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&width.to_le_bytes());
    bytes[22..26].copy_from_slice(&height.to_le_bytes());
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24u16.to_le_bytes());
    bytes[34..38].copy_from_slice(&((stride*height as usize) as u32).to_le_bytes());
    let origin=v(3.7,-6.2,3.6);
    let forward=(v(0.0,0.1,0.8)-origin).normalized().expect("camera forward");
    let right=forward.cross(v(0.0,0.0,1.0)).normalized().expect("camera right");
    let up=right.cross(forward).normalized().expect("camera up");
    let tan_half=(45.0f32.to_radians()*0.5).tan();
    let aspect=width as f32/height as f32;
    for y in 0..height{
        for x in 0..width{
            let px=(2.0*(x as f32+0.5)/width as f32-1.0)*aspect*tan_half;
            let py=(1.0-2.0*(y as f32+0.5)/height as f32)*tan_half;
            let d=(forward+right*px+up*py).normalized().expect("ray direction");
            let ray=Ray::new(origin,d,0.001,1000.0).expect("ray");
            let [r,g,b]=shade(bvh,&ray,y as f32/height as f32);
            let offset=54+(height as usize-1-y as usize)*stride+x as usize*3;
            bytes[offset..offset+3].copy_from_slice(&[b,g,r]);
        }
    }
    bytes
}
fn main()->Result<(),Box<dyn Error>>{
    let args:Vec<String>=std::env::args().collect();
    let filename=args.get(1).map(String::as_str).unwrap_or("circlesrust_preview.bmp");
    let width=args.get(2).map(|s|s.parse::<u32>()).transpose()?.unwrap_or(960);
    let height=args.get(3).map(|s|s.parse::<u32>()).transpose()?.unwrap_or(540);
    if !(1..=4096).contains(&width)||!(1..=4096).contains(&height){
        return Err("Dimensions must be between 1 and 4096".into());
    }
    let bvh=scene();
    fs::write(filename,bmp(&bvh,width,height))?;
    let stats=bvh.stats();
    println!("BMP: {filename} ({width}x{height}). Triangles: {}, BVH nodes: {}, leaves: {}.",
             stats.triangles,stats.nodes,stats.leaves);
    println!("CPU diagnostic raycast only; no Cycles path tracing or GPU.");
    Ok(())
}
