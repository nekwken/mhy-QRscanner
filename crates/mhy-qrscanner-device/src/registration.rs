use chrono::Utc;
use mhy_qrscanner_core::{DeviceProfile, DeviceRegistration};
use mhy_qrscanner_mihoyo::device_api::{DeviceApi, GetFpRequest, MihoyoError};
use mhy_qrscanner_store::file_store::{EncryptedStore, StoreError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("mihoyo api error: {0}")]
    Api(#[from] MihoyoError),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ext list request failed: {0}")]
    ExtList(#[source] MihoyoError),
}

pub async fn register_device(
    api: &dyn DeviceApi,
    store: &EncryptedStore,
    profile: &DeviceProfile,
) -> Result<DeviceRegistration, RegistrationError> {
    let _ext_list = api
        .get_ext_list("2", "bbs_cn")
        .await
        .map_err(RegistrationError::ExtList)?;

    let request = GetFpRequest {
        device_id: profile.device_id.clone(),
        seed_id: profile.seed_id.clone(),
        seed_time: profile.seed_time.clone(),
        platform: "2".to_string(),
        device_fp: profile.initial_device_fp.clone(),
        app_name: "bbs_cn".to_string(),
        ext_fields: serde_json::to_string(&profile.ext_fields)?,
        bbs_device_id: profile.bbs_device_id.clone(),
    };

    let response = api.get_fp(&request).await?;
    let registration = DeviceRegistration {
        device_id: profile.device_id.clone(),
        device_fp: response.device_fp,
        seed_id: profile.seed_id.clone(),
        seed_time: profile.seed_time.clone(),
        bbs_device_id: profile.bbs_device_id.clone(),
        registered_at: Utc::now().to_rfc3339(),
    };

    let key = format!(
        "accounts/{}/device-registration",
        profile.account_id.as_str()
    );
    store.save(&key, &registration)?;
    Ok(registration)
}
