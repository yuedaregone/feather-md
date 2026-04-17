use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 900.0,
            height: 700.0,
            x: 100.0,
            y: 100.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub theme: String,
    pub window: WindowState,
    pub auto_reload: bool,
    #[serde(default)]
    pub file_association_backup: std::collections::HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "github-light".to_string(),
            window: WindowState::default(),
            auto_reload: true,
            file_association_backup: std::collections::HashMap::new(),
        }
    }
}

impl Config {
    fn config_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("FeatherMD"))
    }

    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("config.json"))
    }

    pub fn load() -> Self {
        match Self::config_path() {
            Some(path) if path.exists() => {
                let content = fs::read_to_string(&path).unwrap_or_default();
                serde_json::from_str(&content).unwrap_or_default()
            }
            _ => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir().ok_or("Cannot determine config directory")?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = Self::config_path().ok_or("Cannot determine config path")?;
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    pub fn update_theme(&mut self, theme: &str) {
        self.theme = theme.to_string();
        let _ = self.save();
    }

    pub fn update_window_state(&mut self, window: WindowState) {
        self.window = window;
        let _ = self.save();
    }
}
