use mhy_qrscanner_core::model::{AccountId, GameBiz};
use mhy_qrscanner_device::generator::{generate_profile, name_uuid_from_bytes};
use rand::{rngs::StdRng, SeedableRng};

#[test]
fn device_ids_are_deterministic_for_same_seed() {
    let account = AccountId::new("acct-1").unwrap();
    let mut rng_a = StdRng::seed_from_u64(42);
    let mut rng_b = StdRng::seed_from_u64(42);

    let a = generate_profile(&account, GameBiz::GenshinCn, &mut rng_a);
    let b = generate_profile(&account, GameBiz::GenshinCn, &mut rng_b);

    assert_eq!(a.device_id, b.device_id);
    assert_eq!(a.initial_device_fp, b.initial_device_fp);
    assert_eq!(a.bbs_device_id, b.bbs_device_id);
    assert_eq!(a.model, b.model);
}

#[test]
fn device_ids_have_expected_shapes() {
    let account = AccountId::new("acct-1").unwrap();
    let mut rng = StdRng::seed_from_u64(7);
    let profile = generate_profile(&account, GameBiz::GenshinCn, &mut rng);

    assert_eq!(profile.device_id.len(), 16);
    assert_eq!(profile.initial_device_fp.len(), 13);
    assert_eq!(profile.bbs_device_id.len(), 36);
    assert_eq!(profile.seed_id.len(), 36);
    assert!(profile.seed_time.chars().all(|c| c.is_ascii_digit()));
    assert_eq!(profile.ext_fields["model"], profile.model);
}

#[test]
fn name_uuid_matches_miyoushe_byte_layout() {
    // Reference vector from the archived working implementation
    // (_archive/legacy/python-qmd/qmd/device_fp.py): set the version/variant
    // bits first, then reverse the first three byte segments.
    let uuid = name_uuid_from_bytes(b"0123456789abcdef");
    assert_eq!(uuid.len(), 36);
    assert_eq!(uuid, "8daf3240-0361-2331-906e-58e067140cc5");
}
