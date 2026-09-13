//! Bridge error type.
//!
//! Every variant carries a message the UI can show directly; none of them
//! contains a secret.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("{0}")]
    Invalid(String),
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("server rejected the request (retcode={retcode}): {message}")]
    Api { retcode: i64, message: String },
    /// The user must complete a challenge manually. Never an automated bypass.
    #[error("manual challenge required: {0:?}")]
    Challenge(crate::dto::LoginChallengeDto),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("capture error: {0}")]
    Capture(String),
    #[error("qr error: {0}")]
    Qr(String),
}

impl BridgeError {
    /// Server retcode when this error came from an API envelope.
    pub fn api_retcode(&self) -> Option<i64> {
        match self {
            BridgeError::Api { retcode, .. } => Some(*retcode),
            BridgeError::Challenge(c) => Some(c.retcode),
            _ => None,
        }
    }

    /// Challenge payload when the user must act.
    pub fn challenge(&self) -> Option<&crate::dto::LoginChallengeDto> {
        match self {
            BridgeError::Challenge(c) => Some(c),
            _ => None,
        }
    }
}

impl From<mhy_qrscanner_store::file_store::StoreError> for BridgeError {
    fn from(e: mhy_qrscanner_store::file_store::StoreError) -> Self {
        BridgeError::Storage(e.to_string())
    }
}

impl From<mhy_qrscanner_mihoyo::device_api::MihoyoError> for BridgeError {
    fn from(e: mhy_qrscanner_mihoyo::device_api::MihoyoError) -> Self {
        match e {
            mhy_qrscanner_mihoyo::device_api::MihoyoError::Api { retcode, message } => {
                BridgeError::Api { retcode, message }
            }
            // Fallback for paths that do not run the interactive solver; the
            // auth paths special-case this variant before it can land here.
            mhy_qrscanner_mihoyo::device_api::MihoyoError::AigisChallenge { .. } => BridgeError::Invalid(
                "graphic captcha (aigis) required; retry the request to open the solve page".into(),
            ),
            other => BridgeError::Qr(other.to_string()),
        }
    }
}

impl From<mhy_qrscanner_capture::CaptureError> for BridgeError {
    fn from(e: mhy_qrscanner_capture::CaptureError) -> Self {
        BridgeError::Capture(e.to_string())
    }
}

impl From<mhy_qrscanner_device::registration::RegistrationError> for BridgeError {
    fn from(e: mhy_qrscanner_device::registration::RegistrationError) -> Self {
        BridgeError::Invalid(e.to_string())
    }
}

impl From<mhy_qrscanner_core::CoreError> for BridgeError {
    fn from(e: mhy_qrscanner_core::CoreError) -> Self {
        BridgeError::Invalid(e.to_string())
    }
}
