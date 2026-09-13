use mhy_qrscanner_store::file_store::EncryptedStore;
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Sample {
    device_id: String,
    device_fp: String,
}

#[test]
fn encrypted_store_round_trips_json() {
    let dir = tempdir().unwrap();
    let store = EncryptedStore::new(dir.path());
    let value = Sample {
        device_id: "0123456789abcdef".to_string(),
        device_fp: "abcdef1234567".to_string(),
    };

    store.save("accounts/acct-1/device", &value).unwrap();
    let loaded: Option<Sample> = store.load("accounts/acct-1/device").unwrap();

    assert_eq!(loaded, Some(value));
}

#[test]
fn missing_file_returns_none() {
    let dir = tempdir().unwrap();
    let store = EncryptedStore::new(dir.path());
    let loaded: Option<Sample> = store.load("missing").unwrap();
    assert_eq!(loaded, None);
}

#[test]
fn delete_removes_stored_value() {
    let dir = tempdir().unwrap();
    let store = EncryptedStore::new(dir.path());
    let value = Sample {
        device_id: "0123456789abcdef".to_string(),
        device_fp: "abcdef1234567".to_string(),
    };

    store.save("accounts/acct-1/device", &value).unwrap();
    store.delete("accounts/acct-1/device").unwrap();

    let loaded: Option<Sample> = store.load("accounts/acct-1/device").unwrap();
    assert_eq!(loaded, None);
}

#[test]
fn delete_missing_key_is_ok() {
    let dir = tempdir().unwrap();
    let store = EncryptedStore::new(dir.path());
    store.delete("missing").unwrap();
}
