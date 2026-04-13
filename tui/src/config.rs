use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppConfig {
    pub username: String,
}

impl AppConfig {
    fn file_path() -> PathBuf {
        // Saves in the current directory or home directory
        PathBuf::from(".bump_config.json")
    }

    pub fn load() -> Self {
        fs::read_to_string(Self::file_path())
            .and_then(|content| serde_json::from_str(&content).map_err(|e| e.into()))
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let content =
            serde_json::from_str::<serde_json::Value>(&serde_json::to_string(self).unwrap())
                .unwrap();
        fs::write(
            Self::file_path(),
            serde_json::to_string_pretty(&content).unwrap(),
        )
    }
}
