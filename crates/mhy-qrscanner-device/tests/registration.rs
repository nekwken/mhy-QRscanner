use mhy_qrscanner_core::{AccountId, GameBiz};
use mhy_qrscanner_device::generator::generate_profile;
use mhy_qrscanner_device::registration::register_device;
use mhy_qrscanner_mihoyo::device_api::{DeviceApi, GetFpRequest, GetFpResponse, MihoyoError};
use mhy_qrscanner_store::file_store::EncryptedStore;
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Mutex;
use tempfile::tempdir;

struct FakeApi {
    request: Mutex<Option<GetFpRequest>>,
}

#[async_trait::async_trait]
impl DeviceApi for FakeApi {
    async fn get_ext_list(
        &self,
        _platform: &str,
        _app_name: &str,
    ) -> Result<serde_json::Value, MihoyoError> {
        Ok(serde_json::json!(["model"]))
    }

    async fn get_fp(&self, request: &GetFpRequest) -> Result<GetFpResponse, MihoyoError> {
        *self.request.lock().unwrap() = Some(request.clone());
        Ok(GetFpResponse {
            code: None,
            device_fp: "abcdef1234567".to_string(),
        })
    }
}

#[tokio::test]
async fn register_device_persists_server_fingerprint() {
    let dir = tempdir().unwrap();
    let store = EncryptedStore::new(dir.path());
    let account = AccountId::new("acct-1").unwrap();
    let mut rng = StdRng::seed_from_u64(11);
    let profile = generate_profile(&account, GameBiz::GenshinCn, &mut rng);
    let api = FakeApi {
        request: Mutex::new(None),
    };

    let registration = register_device(&api, &store, &profile).await.unwrap();
    assert_eq!(registration.device_fp, "abcdef1234567");
    assert_eq!(registration.device_id, profile.device_id);

    let loaded: mhy_qrscanner_core::DeviceRegistration = store
        .load("accounts/acct-1/device-registration")
        .unwrap()
        .unwrap();
    assert_eq!(loaded.device_fp, "abcdef1234567");

    let sent = api.request.lock().unwrap().clone().unwrap();
    assert_eq!(sent.device_id, profile.device_id);
    assert_eq!(sent.bbs_device_id, profile.bbs_device_id);
}
