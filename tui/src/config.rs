use dirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub username: String,
    pub default_dir: PathBuf, // New field
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            username: "Anonymous".into(),
            // Default to current dir if we can't find a home
            default_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl AppConfig {
    fn file_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("bump");
        let _ = fs::create_dir_all(&path);
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        fs::read_to_string(Self::file_path())
            .and_then(|content| serde_json::from_str(&content).map_err(|e| e.into()))
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self).unwrap();
        fs::write(Self::file_path(), content)
    }
}
