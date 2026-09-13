use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MihoyoError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("api error retcode={retcode} message={message}")]
    Api { retcode: i64, message: String },
    /// `retcode=-3101`: the server demands a **graphic captcha** (aigis/Geetest)
    /// before it will proceed. The challenge is delivered in the *response
    /// header* `x-rpc-aigis` (the body is empty), captured here verbatim.
    /// This tool never solves it automatically — the user completes it.
    #[error("aigis challenge required")]
    AigisChallenge { challenge: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct GetFpRequest {
    pub device_id: String,
    pub seed_id: String,
    pub seed_time: String,
    pub platform: String,
    pub device_fp: String,
    pub app_name: String,
    pub ext_fields: String,
    pub bbs_device_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GetFpResponse {
    /// Optional inner status code; some responses include a data.code field.
    #[serde(default)]
    pub code: Option<i64>,
    pub device_fp: String,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    retcode: i64,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

#[async_trait]
pub trait DeviceApi: Send + Sync {
    async fn get_ext_list(&self, platform: &str, app_name: &str) -> Result<Value, MihoyoError>;
    async fn get_fp(&self, request: &GetFpRequest) -> Result<GetFpResponse, MihoyoError>;
}

pub struct HttpDeviceApi {
    client: reqwest::Client,
    base_url: String,
}

impl HttpDeviceApi {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl DeviceApi for HttpDeviceApi {
    async fn get_ext_list(&self, platform: &str, app_name: &str) -> Result<Value, MihoyoError> {
        let url = format!(
            "{}/device-fp/api/getExtList?platform={}&app_name={}",
            self.base_url, platform, app_name
        );
        let response = self
            .client
            .get(url)
            .header("User-Agent", "okhttp/4.9.3")
            .send()
            .await?
            .error_for_status()?
            .json::<Envelope<Value>>()
            .await?;

        if response.retcode != 0 {
            return Err(MihoyoError::Api {
                retcode: response.retcode,
                message: response.message,
            });
        }

        response
            .data
            .ok_or_else(|| MihoyoError::InvalidResponse("missing data".to_string()))
    }

    async fn get_fp(&self, request: &GetFpRequest) -> Result<GetFpResponse, MihoyoError> {
        let url = format!("{}/device-fp/api/getFp", self.base_url);
        let response = self
            .client
            .post(url)
            .header("User-Agent", "okhttp/4.9.3")
            .json(request)
            .send()
            .await?
            .error_for_status()?
            .json::<Envelope<GetFpResponse>>()
            .await?;

        if response.retcode != 0 {
            return Err(MihoyoError::Api {
                retcode: response.retcode,
                message: response.message,
            });
        }

        let data = response
            .data
            .ok_or_else(|| MihoyoError::InvalidResponse("missing data".to_string()))?;

        if data.device_fp.is_empty() {
            return Err(MihoyoError::InvalidResponse("empty device_fp".to_string()));
        }
        if matches!(data.code, Some(code) if code != 200) {
            return Err(MihoyoError::InvalidResponse(format!(
                "unexpected data.code={:?}",
                data.code
            )));
        }

        Ok(data)
    }
}
