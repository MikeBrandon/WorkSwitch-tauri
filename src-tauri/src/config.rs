use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub settings: Settings,
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_delay")]
    pub launch_delay_ms: u64,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub close_on_switch: bool,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_delay")]
    pub delay_after: u64,
    #[serde(default)]
    pub process_name: String,
    // App/folder/url fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_running: Option<bool>,
    // Terminal fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_open: Option<bool>,
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_delay() -> u64 {
    500
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            settings: Settings {
                theme: default_theme(),
                launch_delay_ms: 500,
                start_minimized: false,
                close_on_switch: true,
                minimize_to_tray: true,
            },
            profiles: vec![],
        }
    }
}

pub fn config_path() -> PathBuf {
    // Look next to the executable first, then fall back to current dir
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("config.json");
            if p.exists() {
                return p;
            }
            // For dev mode, check the project root
            // In dev mode the exe is in target/debug/
            let dev_path = dir
                .parent()
                .and_then(|d| d.parent())
                .map(|d| d.join("config.json"));
            if let Some(dp) = dev_path {
                if dp.exists() {
                    return dp;
                }
            }
        }
    }
    // Fall back to exe directory for new configs
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("config.json")))
        .unwrap_or_else(|| PathBuf::from("config.json"))
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Failed to parse config: {}", e);
                AppConfig::default()
            }
        },
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;

    // Atomic write: write to temp file, then rename
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &json).map_err(|e| format!("Failed to write config: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename config: {}", e))?;

    Ok(())
}
