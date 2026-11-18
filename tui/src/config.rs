use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub username: String,
    pub default_dir: PathBuf, // New field
    pub palette: Palette,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            username: "Anonymous".into(),
            // Default to current dir if we can't find a home
            default_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            palette: Palette::default(),
        }
    }
}

impl AppConfig {
    pub fn file_path() -> PathBuf {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct Palette {
    pub base00: String, // Default Background
    pub base01: String, // Lighter Background
    pub base02: String, // Selection Background
    pub base03: String, // Comments, Invisibles
    pub base04: String, // Dark Foreground
    pub base05: String, // Default Foreground
    pub base06: String, // Light Foreground
    pub base07: String, // Lightest Foreground
    pub base08: String, // Variables, XML Tags
    pub base09: String, // Integers, Boolean
    pub base0A: String, // Classes, Search Link
    pub base0B: String, // Strings, Inherited Class
    pub base0C: String, // Support, Regular Expressions
    pub base0D: String, // Functions, Methods
    pub base0E: String, // Keywords, Storage
    pub base0F: String, // Deprecated, Opening/Closing Emb. Tags
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            base00: "#282828".into(), // Gruvbox dark
            base01: "#3c3836".into(),
            base02: "#504945".into(),
            base03: "#665c54".into(),
            base04: "#bdae93".into(),
            base05: "#d5c4a1".into(),
            base06: "#ebdbb2".into(),
            base07: "#fbf1c7".into(),
            base08: "#fb4934".into(),
            base09: "#fe8019".into(),
            base0A: "#fabd2f".into(),
            base0B: "#b8bb26".into(),
            base0C: "#8ec07c".into(),
            base0D: "#83a598".into(),
            base0E: "#d3869b".into(),
            base0F: "#d65d0e".into(),
        }
    }
}

impl Palette {
    pub fn get_color(&self, hex: &str) -> Color {
        hex.parse::<Color>().unwrap_or(Color::Reset)
    }
}
