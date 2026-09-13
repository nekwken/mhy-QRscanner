use mhy_qrscanner_mihoyo::ds::{create_sign, DsEnv, Salt};

/// Expected vector from offline Python check of the App formula:
/// salt=SALT_PROD, t=1720000000, r=abcdef,
/// b={"ticket":"tk_abc","token_types":["token"]}
/// md5 = 0448bbdc246ed0c94b99d5c60498d8f0
const PROD_SALT: &str = "JwYDpKvLj6MrMqqYU6jTKF17KNO2PXoS";

#[test]
fn create_sign_is_deterministic_for_fixed_t_r() {
    let params = serde_json::json!({
        "ticket": "tk_abc",
        "token_types": ["token"]
    });
    let ds = create_sign(
        &params,
        Salt::Custom(PROD_SALT.to_string()),
        Some(1_720_000_000),
        Some("abcdef"),
    );
    assert_eq!(ds, "1720000000,abcdef,0448bbdc246ed0c94b99d5c60498d8f0");
}

#[test]
fn create_sign_uses_env_salt() {
    let params = serde_json::json!({"a": 1});
    let prod = create_sign(
        &params,
        Salt::from_env(DsEnv::Product),
        Some(1),
        Some("xxxxxx"),
    );
    let dev = create_sign(&params, Salt::from_env(DsEnv::Dev), Some(1), Some("xxxxxx"));
    assert_ne!(prod, dev);
    assert_eq!(Salt::from_env(DsEnv::Product).as_str().len(), 32);
    assert_eq!(Salt::from_env(DsEnv::Dev).as_str().len(), 32);
}

#[test]
fn random_r_is_six_alnum() {
    let params = serde_json::json!({});
    let ds1 = create_sign(&params, Salt::Custom(PROD_SALT.to_string()), None, None);
    let ds2 = create_sign(&params, Salt::Custom(PROD_SALT.to_string()), None, None);
    for ds in [ds1, ds2] {
        let parts: Vec<_> = ds.split(',').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 6);
        assert!(parts[1].chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(parts[2].len(), 32);
    }
}
