use color_eyre::eyre::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub music_folder: PathBuf,
    #[serde(default)]
    pub play_counts: HashMap<String, u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_folder: PathBuf::from(""),
            play_counts: HashMap::new(),
        }
    }
}

impl Config {
    fn config_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "fernando", "playful") {
            proj_dirs.config_dir().join("config.json")
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config").join("playful").join("config.json")
        }
    }

    pub fn load() -> Option<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(path).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
