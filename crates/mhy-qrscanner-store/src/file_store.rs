use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use zeroize::Zeroizing;

use crate::dpapi;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid storage key: {0}")]
    InvalidKey(String),
}

#[derive(Debug, Clone)]
pub struct EncryptedStore {
    root: PathBuf,
}

impl EncryptedStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, key: &str) -> Result<PathBuf, StoreError> {
        if key.is_empty() || key.contains("..") || key.contains(':') {
            return Err(StoreError::InvalidKey(key.to_string()));
        }
        Ok(self.root.join(format!("{key}.bin")))
    }

    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StoreError> {
        let path = self.path_for(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // `Zeroizing` scrubs the plaintext on every exit path, including the
        // `?` early returns, and its volatile writes survive optimization.
        let plain = Zeroizing::new(serde_json::to_vec(value)?);
        let encrypted = dpapi::protect(&plain)?;
        let tmp = path.with_extension("bin.tmp");
        fs::write(&tmp, &encrypted)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), StoreError> {
        let path = self.path_for(key)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(StoreError::Io(err)),
        }
    }

    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, StoreError> {
        let path = self.path_for(key)?;
        if !path.exists() {
            return Ok(None);
        }
        let encrypted = fs::read(path)?;
        let plain = Zeroizing::new(dpapi::unprotect(&encrypted)?);
        let value = serde_json::from_slice(&plain)?;
        Ok(Some(value))
    }
}
