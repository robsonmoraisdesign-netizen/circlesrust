// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-License-Identifier: Apache-2.0
// Modified from blender/cycles/src/util/hash.h, using Rust wrapping arithmetic.
#[inline]
pub fn uint_to_float_excl(n: u32) -> f32 {
    n as f32 * (1.0 / 4294967808.0f32)
}
#[inline]
pub fn uint_to_float_incl(n: u32) -> f32 {
    n as f32 * (1.0 / u32::MAX as f32)
}
#[inline]
fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*c);
    *a ^= c.rotate_left(4);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= a.rotate_left(6);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= b.rotate_left(8);
    *b = b.wrapping_add(*a);
    *a = a.wrapping_sub(*c);
    *a ^= c.rotate_left(16);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= a.rotate_left(19);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= b.rotate_left(4);
    *b = b.wrapping_add(*a);
}
#[inline]
fn finalize(a: &mut u32, b: &mut u32, c: &mut u32) {
    *c ^= *b;
    *c = c.wrapping_sub(b.rotate_left(14));
    *a ^= *c;
    *a = a.wrapping_sub(c.rotate_left(11));
    *b ^= *a;
    *b = b.wrapping_sub(a.rotate_left(25));
    *c ^= *b;
    *c = c.wrapping_sub(b.rotate_left(16));
    *a ^= *c;
    *a = a.wrapping_sub(c.rotate_left(4));
    *b ^= *a;
    *b = b.wrapping_sub(a.rotate_left(14));
    *c ^= *b;
    *c = c.wrapping_sub(b.rotate_left(24));
}
pub fn hash_uint(x: u32) -> u32 {
    let seed = 0xdead_beefu32.wrapping_add(4 + 13);
    let (mut a, mut b, mut c) = (seed.wrapping_add(x), seed, seed);
    finalize(&mut a, &mut b, &mut c);
    c
}
pub fn hash_uint2(x: u32, y: u32) -> u32 {
    let seed = 0xdead_beefu32.wrapping_add(8 + 13);
    let (mut a, mut b, mut c) = (seed.wrapping_add(x), seed.wrapping_add(y), seed);
    finalize(&mut a, &mut b, &mut c);
    c
}
pub fn hash_uint3(x: u32, y: u32, z: u32) -> u32 {
    let seed = 0xdead_beefu32.wrapping_add(12 + 13);
    let (mut a, mut b, mut c) = (seed.wrapping_add(x), seed.wrapping_add(y), seed.wrapping_add(z));
    finalize(&mut a, &mut b, &mut c);
    c
}
pub fn hash_uint4(x: u32, y: u32, z: u32, w: u32) -> u32 {
    let seed = 0xdead_beefu32.wrapping_add(16 + 13);
    let (mut a, mut b, mut c) = (seed.wrapping_add(x), seed.wrapping_add(y), seed.wrapping_add(z));
    mix(&mut a, &mut b, &mut c);
    a = a.wrapping_add(w);
    finalize(&mut a, &mut b, &mut c);
    c
}
pub fn hash_uint_to_float(x: u32) -> f32 {
    uint_to_float_incl(hash_uint(x))
}
pub fn hash_uint2_to_float(x: u32, y: u32) -> f32 {
    uint_to_float_incl(hash_uint2(x, y))
}
pub fn hash_uint3_to_float(x: u32, y: u32, z: u32) -> f32 {
    uint_to_float_incl(hash_uint3(x, y, z))
}
pub fn hash_uint4_to_float(x: u32, y: u32, z: u32, w: u32) -> f32 {
    uint_to_float_incl(hash_uint4(x, y, z, w))
}
pub fn hash_pcg2d_i(mut v: [i32; 2]) -> [i32; 2] {
    for x in &mut v { *x = x.wrapping_mul(1664525).wrapping_add(1013904223); }
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(1664525));
    v[1] = v[1].wrapping_add(v[0].wrapping_mul(1664525));
    for x in &mut v { *x ^= *x >> 16; }
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(1664525));
    v[1] = v[1].wrapping_add(v[0].wrapping_mul(1664525));
    for x in &mut v { *x &= i32::MAX; }
    v
}
pub fn hash_pcg3d_i(mut v: [i32; 3]) -> [i32; 3] {
    for x in &mut v { *x = x.wrapping_mul(1664525).wrapping_add(1013904223); }
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
    v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
    v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
    for x in &mut v { *x ^= *x >> 16; }
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
    v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
    v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
    for x in &mut v { *x &= i32::MAX; }
    v
}
pub fn hash_pcg4d_i(mut v: [i32; 4]) -> [i32; 4] {
    for x in &mut v { *x = x.wrapping_mul(1664525).wrapping_add(1013904223); }
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[3]));
    v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
    v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
    v[3] = v[3].wrapping_add(v[1].wrapping_mul(v[2]));
    for x in &mut v { *x ^= *x >> 16; }
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[3]));
    v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
    v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
    v[3] = v[3].wrapping_add(v[1].wrapping_mul(v[2]));
    for x in &mut v { *x &= i32::MAX; }
    v
}
pub fn hash_int2_to_float2(v: [i32; 2]) -> [f32; 2] {
    hash_pcg2d_i(v).map(|x| x as f32 / i32::MAX as f32)
}
pub fn hash_int3_to_float3(v: [i32; 3]) -> [f32; 3] {
    hash_pcg3d_i(v).map(|x| x as f32 / i32::MAX as f32)
}
pub fn hash_int4_to_float4(v: [i32; 4]) -> [f32; 4] {
    hash_pcg4d_i(v).map(|x| x as f32 / i32::MAX as f32)
}
