use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub server_url: String,
    pub access_token: String,
    pub rnotes_dir: String,
    pub autostart: bool,
    pub sync_subdirs: bool,
    #[serde(default)]
    pub show_tray_icon: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: "https://notes.huebler.tech".to_string(),
            access_token: String::new(),
            rnotes_dir: dirs::document_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Rnotes")
                .to_string_lossy()
                .to_string(),
            autostart: true,
            sync_subdirs: true,
            show_tray_icon: true,
        }
    }
}

impl AppConfig {
    pub fn path() -> PathBuf {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("syncnotes");
        fs::create_dir_all(&base).ok();
        base.join("config.json")
    }

    pub fn load() -> Option<Self> {
        let path = Self::path();
        if !path.exists() {
            return None;
        }
        let data = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Ok(data) = serde_json::to_string_pretty(self) {
            fs::write(&path, &data).ok();
        }
    }

    pub fn delete() {
        let path = Self::path();
        if path.exists() {
            fs::remove_file(&path).ok();
        }
    }
}
