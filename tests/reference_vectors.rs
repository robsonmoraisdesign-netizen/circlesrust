use circlesrust::kernel::types::{path_dimension, ray_visibility, SamplingPattern};
use circlesrust::util::hash::*;
#[test]
fn jenkins_lookup3_reference_vectors() {
    assert_eq!(hash_uint(0), 2501500005);
    assert_eq!(hash_uint(1), 1255655072);
    assert_eq!(hash_uint(42), 778167589);
    assert_eq!(hash_uint(u32::MAX), 1116397369);
    assert_eq!(hash_uint2(0, 0), 3695015161);
    assert_eq!(hash_uint2(1, 2), 272092061);
    assert_eq!(hash_uint3(0, 0, 0), 2624515579);
    assert_eq!(hash_uint3(1, 2, 3), 3100736010);
}
#[test]
fn float_mapping_range_is_valid() {
    for n in [0, 1, u32::MAX / 2, u32::MAX] {
        assert!((0.0..1.0).contains(&uint_to_float_excl(n)));
        assert!((0.0..=1.0).contains(&uint_to_float_incl(n)));
    }
}
#[test]
fn pcg_hashes_are_deterministic() {
    for v in [[0, 0], [1, -3], [i32::MIN, i32::MAX]] {
        let h = hash_pcg2d_i(v);
        assert_eq!(h, hash_pcg2d_i(v));
        assert!(h.iter().all(|x| *x >= 0));
    }
    assert_eq!(hash_pcg3d_i([1, 2, 3]), hash_pcg3d_i([1, 2, 3]));
    assert_eq!(hash_pcg4d_i([1, 2, 3, 4]), hash_pcg4d_i([1, 2, 3, 4]));
}
#[test]
fn constants_match_reference() {
    assert_eq!(SamplingPattern::SobolBurley as u32, 0);
    assert_eq!(SamplingPattern::Automatic as u32, 5);
    assert_eq!(path_dimension::PRNG_BOUNCE_NUM, 16);
    assert_eq!(ray_visibility::ALL, 127);
    assert_eq!(ray_visibility::SHADOW, 96);
}
