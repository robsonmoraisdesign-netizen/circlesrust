    use super::*;
    #[test]
    fn environment_and_bsdf_mis_agree_with_independent_spherical_quadrature() {
        let map=ImageTexture::from_rgb32f(8,4,&[1.0;8*4*3]).unwrap();
        let dist=map.environment_distribution();
        let n=Vec3::new(0.0,0.0,1.0);
        let view=n;
        for material in [Material::new(Vec3::new(0.7,0.5,0.3),0.0,0.6),
            Material::new(Vec3::new(0.7,0.5,0.3),1.0,0.6)] {
            let mut reference=0.0f64;
            for y in 0..128 { for x in 0..256 {
                let z=(y as f32+0.5)/128.0;
                let phi=2.0*PI*(x as f32+0.5)/256.0;
                let r=(1.0-z*z).sqrt();
                let d=Vec3::new(r*phi.cos(),r*phi.sin(),z);
                reference+=bsdf_eval(material,n,view,d).0.x as f64*z as f64;
            }}
            reference*=2.0*std::f64::consts::PI/(128.0*256.0);
            let mut rng=Rng::new(0,0,13);
            let mut estimate=0.0f64;
            for _ in 0..80000 {
                let (d,p)=dist.sample([rng.next_f32(),rng.next_f32(),rng.next_f32()]).unwrap();
                let (f,b)=bsdf_eval(material,n,view,d);
                estimate+=(f.x*n.dot(d).max(0.0)*power_heuristic(p,b)/p) as f64;
                if let Some((d,f,weight,b))=bsdf_sample(material,n,view,&mut rng){
                    estimate+=(f.x*weight*power_heuristic(b,dist.pdf(d))) as f64;
                }
            }
            estimate/=80000.0;
            assert!((estimate-reference).abs()/reference<0.02,
                "MIS estimate {estimate}, independent quadrature {reference}");
        }
    }
