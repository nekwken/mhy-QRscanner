//! Core handle and construction.
//!
//! `Core` is opaque to Dart (`#[frb(opaque)]`), so construction happens through
//! the free functions here rather than a Dart-side constructor.

use crate::dto::BridgeInfo;
use flutter_rust_bridge::frb;
use mhy_qrscanner_store::file_store::EncryptedStore;
use std::path::PathBuf;

/// Shared entry point handed to the UI.
///
/// Owns the encrypted store root and the hosts used for network calls. Hosts are
/// injectable so tests can point at a mock server.
#[frb(opaque)]
#[derive(Debug, Clone)]
pub struct Core {
    store: EncryptedStore,
    data_dir: PathBuf,
    pub(crate) passport_host: String,
    panda_host_override: Option<String>,
    pub(crate) device_api_host: String,
}

impl Core {
    /// `data_dir` is where DPAPI-encrypted profiles and sessions live.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        Self {
            store: EncryptedStore::new(data_dir.clone()),
            data_dir,
            passport_host: mhy_qrscanner_qr::PASSPORT_BASE.to_string(),
            panda_host_override: None,
            device_api_host: "https://public-data-api.mihoyo.com".to_string(),
        }
    }

    pub fn with_passport_host(mut self, host: impl Into<String>) -> Self {
        self.passport_host = host.into();
        self
    }

    pub fn with_panda_host(mut self, host: impl Into<String>) -> Self {
        self.panda_host_override = Some(host.into());
        self
    }

    pub fn with_device_api_host(mut self, host: impl Into<String>) -> Self {
        self.device_api_host = host.into();
        self
    }

    pub(crate) fn store(&self) -> &EncryptedStore {
        &self.store
    }

    /// Root of the store, also where `audit.log` lives.
    pub(crate) fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    pub(crate) fn passport_host(&self) -> &str {
        &self.passport_host
    }

    pub(crate) fn device_api_host(&self) -> &str {
        &self.device_api_host
    }

    /// panda host for a `game_biz`: explicit override wins, else derived.
    pub(crate) fn panda_host_for(&self, game_biz: &str) -> String {
        self.panda_host_override
            .clone()
            .unwrap_or_else(|| mhy_qrscanner_qr::panda_host_for(game_biz))
    }

    /// Describe the core to the UI (about/settings screen).
    pub fn info(&self) -> BridgeInfo {
        BridgeInfo {
            core_version: env!("CARGO_PKG_VERSION").to_string(),
            data_dir: self.data_dir.display().to_string(),
            passport_host: self.passport_host.clone(),
            device_api_host: self.device_api_host.clone(),
            games: mhy_qrscanner_core::GameBiz::CN_ALL
                .iter()
                .map(|g| g.as_str().to_string())
                .collect(),
            device_templates: crate::api::device::TEMPLATE_NAMES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

/// Default per-user data directory (`%LOCALAPPDATA%\mhy\QRscanner`).
pub fn default_data_dir() -> String {
    directories::ProjectDirs::from("dev", "mhy", "QRscanner")
        .map(|d| d.data_local_dir().display().to_string())
        .unwrap_or_else(|| ".".to_string())
}

/// Create the core handle at the default per-user data directory.
pub fn core_new() -> Core {
    Core::new(default_data_dir())
}

/// Create the core handle at an explicit directory (alternate profile, tests).
pub fn core_new_at(data_dir: String) -> Core {
    Core::new(data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_new_at_uses_given_dir() {
        let dir = tempfile::tempdir().unwrap();
        let core = core_new_at(dir.path().display().to_string());
        let info = core.info();
        assert_eq!(info.data_dir, dir.path().display().to_string());
        // the verified game list and templates are advertised
        assert!(info.games.contains(&"hk4e_cn".to_string()));
        assert!(info.device_templates.contains(&"xiaomi14".to_string()));
    }

    #[test]
    fn default_data_dir_is_not_empty() {
        assert!(!default_data_dir().is_empty());
    }
}
