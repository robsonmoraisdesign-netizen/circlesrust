struct Camera { vp:mat4x4<f32>, light:vec4<f32>, eye:vec4<f32>, selection:vec4<f32> }
struct Object { model:mat4x4<f32>, normal:mat4x4<f32> }
@group(0) @binding(0) var<uniform> camera:Camera;
@group(0) @binding(1) var<storage,read> objects:array<Object>;
struct Vertex {
 @location(0) position:vec3<f32>, @location(1) normal:vec3<f32>,
 @location(2) color:vec3<f32>, @location(3) object:u32
}
struct Pixel {
 @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>,
 @location(1) normal:vec3<f32>, @location(2) color:vec3<f32>,
 @location(3) @interpolate(flat) object:u32
}
@vertex fn vertex(v:Vertex)->Pixel {
 var out:Pixel;
 let model=objects[v.object];
 let p=model.model*vec4<f32>(v.position,1.0);
 out.clip=camera.vp*p;out.world=p.xyz;
 out.normal=(model.normal*vec4<f32>(v.normal,0.0)).xyz;
 out.color=v.color;out.object=v.object;
 return out;
}
@fragment fn fragment(v:Pixel,@builtin(front_facing) front:bool)->@location(0) vec4<f32> {
 let normal=normalize(v.normal)*select(-1.0,1.0,front);
 let light=normalize(camera.light.xyz-v.world);
 let eye=normalize(camera.eye.xyz-v.world);
 let diffuse=max(dot(normal,light),0.0);
 var color=v.color*(vec3<f32>(0.26,0.29,0.35)+vec3<f32>(0.9,0.88,0.84)*diffuse*0.75);
 color+=v.color*pow(max(dot(normal,normalize(light+eye)),0.0),20.0)*0.08;
 if f32(v.object)==camera.selection.w { color=color*0.55+camera.selection.xyz*0.45; }
 color=clamp(color,vec3<f32>(0.0),vec3<f32>(1.0));
 if camera.eye.w<0.5 { color=pow(color,vec3<f32>(1.0/2.2)); }
 return vec4<f32>(color,1.0);
}
