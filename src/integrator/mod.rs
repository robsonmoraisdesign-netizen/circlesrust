// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
//! A small native Rust CPU path integration milestone.
//! Derived in approach from clonecircles/src/kernel/{integrator,closure,light,sample}.
//! Not a full port of the Cycles shader graph, sampling system or device kernels.

use std::f32::consts::PI;
use std::thread;
use crate::bvh::Bvh;
use crate::kernel::geometry::{Ray, Triangle};
use crate::util::hash::hash_uint3;
use crate::util::math::Vec3;

fn rgb_mul(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}
fn max_color(a: Vec3) -> f32 {
    a.x.max(a.y).max(a.z)
}
fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a * (1.0 - t) + b * t
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub base_color: Vec3,
    pub metallic: f32,
    pub roughness: f32,
}
impl Material {
    pub const fn new(base_color: Vec3, metallic: f32, roughness: f32) -> Self {
        Self { base_color, metallic, roughness }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RectLight {
    pub center: Vec3,
    pub half_u: Vec3,
    pub half_v: Vec3,
    /// Emitted radiance in linear RGB; front side follows half_u cross half_v.
    pub radiance: Vec3,
}
impl RectLight {
    fn normal(self) -> Vec3 {
        self.half_u.cross(self.half_v).normalized().unwrap_or(Vec3::ZERO)
    }
    fn area(self) -> f32 {
        4.0 * self.half_u.cross(self.half_v).length_squared().sqrt()
    }
    fn sample(self, rng: &mut Rng) -> Vec3 {
        self.center + self.half_u * (2.0 * rng.next_f32() - 1.0)
                    + self.half_v * (2.0 * rng.next_f32() - 1.0)
    }
}

pub struct Scene {
    pub bvh: Bvh,
    pub triangle_materials: Vec<usize>,
    /// Per-triangle optional corner normals (aligned with input triangle indexing).
    /// Equivalent in intent to Cycles triangle_smooth_normal barycentric interpolation.
    pub corner_normals: Vec<Option<[Vec3; 3]>>,
    pub materials: Vec<Material>,
    pub light: RectLight,
    pub sky_strength: f32,
}
impl Scene {
    pub fn new(triangles: Vec<Triangle>, triangle_materials: Vec<usize>,
               materials: Vec<Material>, light: RectLight) -> Result<Self, &'static str> {
        if triangles.len() != triangle_materials.len()
            || materials.is_empty()
            || triangle_materials.iter().any(|&i| i >= materials.len())
            || !light.center.is_finite()
            || !light.half_u.is_finite()
            || !light.half_v.is_finite()
            || !light.radiance.is_finite()
            || light.area() <= 0.0
        {
            return Err("Invalid scene materials, triangle mapping or rectangle light");
        }
        Ok(Self {
            bvh: Bvh::build(triangles),
            corner_normals: vec![None; triangle_materials.len()],
            triangle_materials, materials,
            light, sky_strength: 1.0,
        })
    }
    /// Install optional corner normals, preserving geometric normals on flat faces.
    pub fn set_corner_normals(&mut self, normals: Vec<Option<[Vec3;3]>>) -> Result<(), &'static str> {
        if normals.len() != self.triangle_materials.len() ||
            normals.iter().flatten().any(|ns| ns.iter().any(|n| !n.is_finite() || n.length_squared() <= 0.0)) {
            return Err("invalid corner normals");
        }
        self.corner_normals = normals;
        Ok(())
    }

    pub fn shading_normal(&self, triangle: usize, u: f32, v: f32) -> Vec3 {
        let ng = self.bvh.triangle(triangle).expect("valid hit index").normal();
        if let Some([a,b,c]) = self.corner_normals.get(triangle).and_then(|x| *x) {
            let interpolated = a * (1.0 - u - v) + b * u + c * v;
            if let Some(smooth) = interpolated.normalized() {
                // Avoid a physically invalid shading normal below the geometric surface.
                if smooth.dot(ng) > 0.0 { return smooth; }
            }
        }
        ng
    }

    fn material_for(&self, triangle: usize) -> Material {
        self.materials[self.triangle_materials[triangle]]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RenderSettings {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub max_bounces: u32,
    pub exposure: f32,
    pub seed: u32,
}
impl Default for RenderSettings {
    fn default() -> Self {
        Self { width: 960, height: 540, samples: 64, max_bounces: 5, exposure: 1.2, seed: 17 }
    }
}
impl RenderSettings {
    pub fn validate(self) -> Result<Self, &'static str> {
        if !(1..=4096).contains(&self.width)
            || !(1..=4096).contains(&self.height)
            || !(1..=1024).contains(&self.samples)
            || !(1..=32).contains(&self.max_bounces)
            || !self.exposure.is_finite() || self.exposure <= 0.0
        {
            return Err("Invalid render settings: width/height 1..4096, samples 1..1024, bounces 1..32");
        }
        Ok(self)
    }
}

#[derive(Clone, Copy)]
struct Rng { state: u32 }
impl Rng {
    fn new(pixel: u32, sample: u32, seed: u32) -> Self {
        Self { state: hash_uint3(pixel, sample, seed).max(1) }
    }
    fn next_u32(&mut self) -> u32 {
        // PCG RXS-M-XS: Cycles also uses PCG hashing for sample scrambling.
        self.state = self.state.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
        let shift = (self.state >> 28) + 4;
        let word = ((self.state >> shift) ^ self.state).wrapping_mul(277_803_737);
        (word >> 22) ^ word
    }
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

fn basis(n: Vec3) -> (Vec3, Vec3) {
    let axis = if n.z.abs() < 0.999 { Vec3::new(0.0, 0.0, 1.0) }
               else { Vec3::new(0.0, 1.0, 0.0) };
    let t = axis.cross(n).normalized().unwrap_or(Vec3::new(1.0, 0.0, 0.0));
    (t, n.cross(t))
}
fn to_world(local: Vec3, n: Vec3) -> Vec3 {
    let (t, b) = basis(n);
    (t * local.x + b * local.y + n * local.z).normalized().unwrap_or(n)
}
fn cosine_hemisphere(n: Vec3, rng: &mut Rng) -> Vec3 {
    // Importance sampling of the Cycles diffuse BSDF (cos(theta)/pi).
    let u = rng.next_f32();
    let phi = 2.0 * PI * rng.next_f32();
    let r = u.sqrt();
    to_world(Vec3::new(r * phi.cos(), r * phi.sin(), (1.0 - u).sqrt()), n)
}
fn ggx_half_vector(n: Vec3, roughness: f32, rng: &mut Rng) -> Vec3 {
    let alpha = roughness.max(0.045).powi(2);
    let a2 = alpha * alpha;
    let u = rng.next_f32();
    let cos_theta = ((1.0-u)/(1.0 + (a2-1.0)*u)).sqrt();
    let phi = 2.0 * PI * rng.next_f32();
    let sin_theta = (1.0-cos_theta*cos_theta).max(0.0).sqrt();
    to_world(Vec3::new(sin_theta*phi.cos(),sin_theta*phi.sin(),cos_theta), n)
}
fn fresnel_schlick(f0: Vec3, vdh: f32) -> Vec3 {
    f0 + (Vec3::new(1.0,1.0,1.0) - f0) * (1.0-vdh.clamp(0.0,1.0)).powi(5)
}
fn ggx_distribution(ndh: f32, roughness: f32) -> f32 {
    let alpha = roughness.max(0.045).powi(2);
    let a2 = alpha * alpha;
    let denom = ndh*ndh*(a2-1.0)+1.0;
    a2 / (PI * denom * denom).max(1e-12)
}
fn geometry_smith(ndv: f32, ndl: f32, roughness: f32) -> f32 {
    let k = (roughness + 1.0).powi(2) / 8.0;
    let g1 = |n:f32| n / (n*(1.0-k)+k).max(1e-8);
    g1(ndv)*g1(ndl)
}
fn glossy_probability(metallic: f32) -> f32 {
    (0.30 + 0.55 * metallic).clamp(0.05, 0.95)
}

/// Returns BSDF f (not f*cos) and mixture sampling pdf in solid angle.
fn bsdf_eval(material: Material, n: Vec3, view: Vec3, incoming: Vec3) -> (Vec3, f32) {
    let ndv = n.dot(view).max(0.0);
    let ndl = n.dot(incoming).max(0.0);
    if ndv <= 0.0 || ndl <= 0.0 { return (Vec3::ZERO, 0.0); }
    let h = match (view + incoming).normalized() {
        Some(h) => h, None => return (Vec3::ZERO,0.0),
    };
    let ndh = n.dot(h).max(0.0);
    let vdh = view.dot(h).max(0.0);
    if vdh <= 0.0 { return (Vec3::ZERO,0.0); }

    let metallic = material.metallic.clamp(0.0,1.0);
    let roughness = material.roughness.clamp(0.045,1.0);
    let base = material.base_color;
    let f0 = mix(Vec3::new(0.04,0.04,0.04),base,metallic);
    let fresnel = fresnel_schlick(f0,vdh);
    let d = ggx_distribution(ndh,roughness);
    let g = geometry_smith(ndv,ndl,roughness);
    let specular = fresnel * (d*g/(4.0*ndv*ndl).max(1e-8));
    let diffuse = rgb_mul(base, Vec3::new(1.0,1.0,1.0)-fresnel) * ((1.0-metallic)/PI);
    let p_spec = glossy_probability(metallic);
    let pdf_spec = d*ndh/(4.0*vdh).max(1e-8);
    let pdf = (1.0-p_spec)*(ndl/PI)+p_spec*pdf_spec;
    (diffuse+specular, pdf)
}
fn bsdf_sample(material:Material,n:Vec3,view:Vec3,rng:&mut Rng)->Option<(Vec3,Vec3,f32)> {
    let incoming = if rng.next_f32() < glossy_probability(material.metallic.clamp(0.0,1.0)){
        let h=ggx_half_vector(n,material.roughness,rng);
        let vdh=view.dot(h);
        if vdh<=0.0 {return None;}
        (h*(2.0*vdh)-view).normalized()?
    } else {
        cosine_hemisphere(n,rng)
    };
    let cos=n.dot(incoming);
    if cos<=0.0 {return None;}
    let (f,pdf)=bsdf_eval(material,n,view,incoming);
    if pdf<=1e-12 || !pdf.is_finite() {return None;}
    Some((incoming,f,cos/pdf))
}

fn environment(direction: Vec3, sky_strength: f32) -> Vec3 {
    let t = (direction.z*0.5 + 0.5).clamp(0.0,1.0);
    let lower = Vec3::new(0.15,0.16,0.18);
    let upper = Vec3::new(0.42,0.55,0.80);
    mix(lower,upper,t) * sky_strength
}
fn direct_light(scene:&Scene,point:Vec3,n:Vec3,ng:Vec3,view:Vec3,material:Material,rng:&mut Rng)->Vec3{
    let light=scene.light;
    let target=light.sample(rng);
    let delta=target-point;
    let distance2=delta.length_squared();
    if distance2<=1e-6{return Vec3::ZERO;}
    let distance=distance2.sqrt();
    let direction=delta/distance;
    let ndl=n.dot(direction);
    let emit_cos=light.normal().dot(-direction);
    if ndl<=0.0 || emit_cos<=0.0{return Vec3::ZERO;}
    // Ray bounds terminate before the emitter to prevent false self-occlusion.
    let Some(shadow)=Ray::new(point+ng*0.0015,direction,0.0005,distance-0.003) else {
        return Vec3::ZERO;
    };
    if scene.bvh.occluded(&shadow){return Vec3::ZERO;}
    let (f,_pdf)=bsdf_eval(material,n,view,direction);
    let geometry=(ndl*emit_cos*light.area()/distance2).max(0.0);
    rgb_mul(f,light.radiance)*geometry
}
fn trace(scene:&Scene,first:Ray,max_bounces:u32,rng:&mut Rng)->Vec3{
    let mut throughput=Vec3::new(1.0,1.0,1.0);
    let mut radiance=Vec3::ZERO;
    let mut ray=first;
    for bounce in 0..max_bounces{
        let Some(hit)=scene.bvh.closest_hit(&ray) else{
            radiance=radiance+rgb_mul(throughput,environment(ray.direction,scene.sky_strength));
            break;
        };
        let material=scene.material_for(hit.triangle_index);
        let mut geometric = scene.bvh.triangle(hit.triangle_index).expect("valid triangle id").normal();
        let mut normal = scene.shading_normal(hit.triangle_index, hit.u, hit.v);
        if geometric.dot(ray.direction) > 0.0 {
            geometric = -geometric;
            normal = -normal;
        }
        let view=-ray.direction;
        let point=hit.point(&ray);
        radiance=radiance+rgb_mul(throughput,direct_light(scene,point,normal,geometric,view,material,rng));
        let Some((next,f,weight))=bsdf_sample(material,normal,view,rng) else {break;};
        throughput=rgb_mul(throughput,f)*weight;
        if !throughput.is_finite() || max_color(throughput)<=0.0{break;}
        if bounce>=2{
            let survival=max_color(throughput).clamp(0.10,0.95);
            if rng.next_f32()>=survival{break;}
            throughput=throughput/survival;
        }
        let Some(next_ray)=Ray::new(point+geometric*0.0015,next,0.0005,1000.0) else{break;};
        ray=next_ray;
    }
    radiance
}

#[derive(Clone, Copy)]
struct Camera {
    origin:Vec3,
    forward:Vec3,
    right:Vec3,
    up:Vec3,
    tangent:f32,
}
impl Camera{
    fn studio()->Self{
        let origin=Vec3::new(4.1,-7.6,3.65);
        let target=Vec3::new(0.0,0.15,0.96);
        let forward=(target-origin).normalized().expect("camera forward");
        let right=forward.cross(Vec3::new(0.0,0.0,1.0)).normalized().expect("camera right");
        let up=right.cross(forward);
        Self{origin,forward,right,up,tangent:(40.0f32.to_radians()*0.5).tan()}
    }
    fn ray(self,x:u32,y:u32,options:RenderSettings,rng:&mut Rng)->Ray{
        let dx=(x as f32+rng.next_f32())/options.width as f32;
        let dy=(y as f32+rng.next_f32())/options.height as f32;
        let aspect=options.width as f32/options.height as f32;
        let px=(2.0*dx-1.0)*aspect*self.tangent;
        let py=(1.0-2.0*dy)*self.tangent;
        let direction=(self.forward+self.right*px+self.up*py).normalized().expect("camera ray");
        Ray::new(self.origin,direction,0.001,1000.0).expect("valid camera ray")
    }
}
fn linear_to_srgb(v:f32)->f32{
    if v<=0.0031308{12.92*v}else{1.055*v.powf(1.0/2.4)-0.055}
}
fn display_byte(linear:f32,exposure:f32)->u8{
    let x=(linear*exposure).max(0.0);
    // ACES filmic fit (Narkowicz). Similar intent to Cycles display transforms,
    // not an implementation of OCIO/AgX.
    let mapped=((x*(2.51*x+0.03))/(x*(2.43*x+0.59)+0.14)).clamp(0.0,1.0);
    (linear_to_srgb(mapped).clamp(0.0,1.0)*255.0+0.5) as u8
}

/// CPU multithreaded Monte Carlo renderer. Returns packed, top-down RGB pixels.
/// Each pixel gets independently seeded samples, regardless of worker count.
pub fn render_rgb(scene:&Scene,settings:RenderSettings)->Result<Vec<u8>,&'static str>{
    let settings=settings.validate()?;
    let width=settings.width as usize;
    let height=settings.height as usize;
    let mut bytes=vec![0u8;width*height*3];
    let camera=Camera::studio();
    let workers=thread::available_parallelism().map(|x|x.get()).unwrap_or(1).min(height);
    let chunk_rows=height.div_ceil(workers);
    thread::scope(|scope|{
        for (chunk_index,chunk) in bytes.chunks_mut(chunk_rows*width*3).enumerate(){
            scope.spawn(move||{
                let base_y=chunk_index*chunk_rows;
                for (local_y,row) in chunk.chunks_exact_mut(width*3).enumerate(){
                    let y=(base_y+local_y) as u32;
                    for (x,pixel) in row.as_chunks_mut::<3>().0.iter_mut().enumerate(){
                        let mut sum=Vec3::ZERO;
                        for s in 0..settings.samples{
                            let mut rng=Rng::new(y.wrapping_mul(settings.width).wrapping_add(x as u32),s,settings.seed);
                            let ray=camera.ray(x as u32,y,settings,&mut rng);
                            sum=sum+trace(scene,ray,settings.max_bounces,&mut rng);
                        }
                        let average=sum/settings.samples as f32;
                        pixel[0]=display_byte(average.x,settings.exposure);
                        pixel[1]=display_byte(average.y,settings.exposure);
                        pixel[2]=display_byte(average.z,settings.exposure);
                    }
                }
            });
        }
    });
    Ok(bytes)
}

/// Check emitted rays and light transport for one deterministic camera pixel.
/// Useful for quality regression checks without allocating a full film buffer.
pub fn sample_pixel(scene:&Scene,settings:RenderSettings,x:u32,y:u32)->Result<Vec3,&'static str>{
    let s=settings.validate()?;
    if x>=s.width||y>=s.height{return Err("pixel out of image bounds");}
    let mut sum=Vec3::ZERO;
    for sample in 0..s.samples{
        let mut rng=Rng::new(y.wrapping_mul(s.width).wrapping_add(x),sample,s.seed);
        let ray=Camera::studio().ray(x,y,s,&mut rng);
        sum=sum+trace(scene,ray,s.max_bounces,&mut rng);
    }
    Ok(sum/s.samples as f32)
}
