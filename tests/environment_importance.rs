use circlesrust::{shader::ImageTexture, util::math::Vec3};
use std::f64::consts::{PI,TAU};
fn direction(u:f64,z:f64)->Vec3{
    let phi=TAU*(u-0.5);let r=(1.0-z*z).max(0.0).sqrt();
    Vec3::new((r*phi.cos()) as f32,(r*phi.sin()) as f32,z as f32)
}
fn random(state:&mut u32)->f32{
    *state=state.wrapping_mul(1664525).wrapping_add(1013904223);
    (*state>>8) as f32/16777216.0
}
#[test]
fn constant_and_black_maps_have_uniform_spherical_pdf_including_poles(){
    for value in [0.0,1.0,1.0e30]{
        let map=ImageTexture::from_rgb32f(16,8,&vec![value;16*8*3]).unwrap();
        let dist=map.environment_distribution();
        for dir in [Vec3::new(0.0,0.0,1.0),Vec3::new(0.0,0.0,-1.0),Vec3::new(-1.0,0.0,0.0)]{
            assert!((dist.pdf(dir) as f64-1.0/(4.0*PI)).abs()<1e-6);
        }
        for r in [[0.0,0.0,0.0],[1.0,1.0,1.0],[0.3,0.5,0.6]]{
            let (dir,pdf)=dist.sample(r).unwrap();
            assert!(dir.is_finite()&&pdf.is_finite()&&pdf>0.0);
            assert!((dir.length_squared()-1.0).abs()<1e-6);
            assert_eq!(pdf,dist.pdf(dir));
        }
        assert_eq!(dist.pdf(Vec3::ZERO),0.0);
        assert_eq!(dist.pdf(Vec3::new(f32::NAN,0.0,0.0)),0.0);
        assert!(dist.sample([f32::NAN,0.5,0.5]).is_none());
        assert!(dist.sample([-0.1,0.5,0.5]).is_none());
    }
}
#[test]
fn pdf_integrates_to_one_and_hotspot_is_selected_more_often(){
    let mut pixels=vec![0.0;32*16*3];
    pixels[(5*32+12)*3..(5*32+12)*3+3].fill(1000.0);
    let map=ImageTexture::from_rgb32f(32,16,&pixels).unwrap();
    let dist=map.environment_distribution();
    let mut integral=0.0;
    for y in 0..16{
        let top=(PI*y as f64/16.0).cos();let bottom=(PI*(y+1) as f64/16.0).cos();
        for x in 0..32{
            integral+=dist.pdf(direction((x as f64+0.5)/32.0,(top+bottom)*0.5)) as f64*TAU/32.0*(top-bottom);
        }
    }
    assert!((integral-1.0).abs()<1e-6,"integral {integral}");
    let mut seed=7;let mut hot=0;
    for _ in 0..20000{
        let (d,p)=dist.sample([random(&mut seed),random(&mut seed),random(&mut seed)]).unwrap();
        assert!(p>0.0&&p.is_finite());
        if p>10.0{hot+=1;}
        assert_eq!(p,dist.pdf(d));
    }
    assert!(hot>18500,"hot samples {hot}");
}
#[test]
fn importance_estimator_preserves_energy_and_reduces_hotspot_variance(){
    let mut pixels=vec![0.02;64*32*3];
    for y in 10..14{for x in 20..24{pixels[(y*64+x)*3..(y*64+x)*3+3].fill(40.0);}}
    let map=ImageTexture::from_rgb32f(64,32,&pixels).unwrap();
    let dist=map.environment_distribution();
    // Independent uniform-solid-angle quadrature integrates the actual bilinear lookup.
    let mut reference=0.0;
    for y in 0..256{for x in 0..512{
        reference+=map.sample_latlong(direction((x as f64+0.5)/512.0,1.0-2.0*(y as f64+0.5)/256.0)).x as f64;
    }}
    reference*=4.0*PI/(256.0*512.0);
    let mut seed=73;let mut sum=0.0;let mut importance_error=0.0;let mut uniform_error=0.0;
    for _ in 0..100000{
        let (d,p)=dist.sample([random(&mut seed),random(&mut seed),random(&mut seed)]).unwrap();
        let estimate=map.sample_latlong(d).x as f64/p as f64;
        sum+=estimate;importance_error+=(estimate-reference).powi(2);
        let u=direction(random(&mut seed) as f64,1.0-2.0*random(&mut seed) as f64);
        let uniform=map.sample_latlong(u).x as f64*4.0*PI;
        uniform_error+=(uniform-reference).powi(2);
    }
    let mean=sum/100000.0;
    assert!((mean-reference).abs()/reference<0.03,"mean {mean}, reference {reference}");
    assert!(importance_error<uniform_error*0.5,"importance {importance_error}, uniform {uniform_error}");
}
#[test]
fn single_pixel_and_large_nondivisible_map_remain_finite_and_normalized(){
    let small=ImageTexture::from_rgb32f(1,1,&[4.0,2.0,1.0]).unwrap();
    assert!((small.environment_distribution().pdf(Vec3::new(1.0,0.0,0.0)) as f64-1.0/(4.0*PI)).abs()<1e-6);
    let mut data=vec![0.0;513*257*3];data[3*123..3*123+3].fill(f32::MAX);
    let large=ImageTexture::from_rgb32f(513,257,&data).unwrap();
    let distribution=large.environment_distribution();let mut sum=0.0;
    for y in 0..256{
        let top=(PI*y as f64/256.0).cos();let bottom=(PI*(y+1) as f64/256.0).cos();
        for x in 0..512{sum+=distribution.pdf(direction((x as f64+0.5)/512.0,(top+bottom)*0.5)) as f64*TAU/512.0*(top-bottom);}
    }
    assert!((sum-1.0).abs()<0.0001,"integral {sum}");
}
