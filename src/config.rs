use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnchorConfig {
    pub edge_bottom: bool,
    pub edge_right: bool,
    pub edge_top: bool,
    pub edge_left: bool,
    pub margin_bottom: i32,
    pub margin_right: i32,
    pub margin_top: i32,
    pub margin_left: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PetConfig {
    pub scale: f64,
    pub size: i32,
    pub cursor_speed_threshold: f64,
    pub window_interaction: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PomodoroConfig {
    pub work_duration_mins: u32,
    pub short_break_mins: u32,
    pub long_break_mins: u32,
    pub start_paused: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub anchor: AnchorConfig,
    pub pet: PetConfig,
    pub pomodoro: PomodoroConfig,
    pub ha_address: String,
    pub ha_key: String,
    pub texture: Vec<LayerEntry>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ColorMode {
    Static {
        r: f64,
        g: f64,
        b: f64,
    },
    Linear {
        angle_deg: f64,
        stops: Vec<(f64, f64, f64, f64)>,
    },
    Radial {
        stops: Vec<(f64, f64, f64, f64)>,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LayerEntry {
    pub color: ColorMode,
    pub layer: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            anchor: AnchorConfig {
                edge_bottom: true,
                edge_right: true,
                edge_top: false,
                edge_left: false,
                margin_bottom: 20,
                margin_right: 20,
                margin_top: 0,
                margin_left: 0,
            },
            pet: PetConfig {
                scale: 1.0,
                size: 128,
                cursor_speed_threshold: 30.0,
                window_interaction: true,
            },
            pomodoro: PomodoroConfig {
                work_duration_mins: 25,
                short_break_mins: 5,
                long_break_mins: 15,
                start_paused: true,
            },
            texture: vec![LayerEntry {
                color: ColorMode::Static {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                },
                layer: "Lineart".to_string(),
            }, LayerEntry {
                color: ColorMode::Static { r: 250.0, g: 150.0, b: 40.0 },
                layer: "BaseColor".to_string()
            }],
            ha_address: "".to_string(),
            ha_key: "".to_string(),
        }
    }
}

pub fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME").expect("Home should be defined");
    PathBuf::from(home).join(".config/pet-app/config.toml")
}

pub fn load_or_create_config() -> Config {
    let path = get_config_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let default_config = Config::default();
        if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
            let _ = fs::write(&path, toml_str);
        }
        default_config
    } else {
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    log::warn!("Error parsing config: {}, using default", e);
                    Config::default()
                }
            },
            Err(e) => {
                log::warn!("Error reading config: {}, using default", e);
                Config::default()
            }
        }
    }
}

pub fn watch_config<F>(callback: F) -> Result<RecommendedWatcher, notify::Error>
where
    F: Fn() + Send + 'static,
{
    let path = get_config_path();

    let _ = load_or_create_config();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| match res {
        Ok(event) => {
            if event.kind.is_modify() {
                callback();
            }
        }
        Err(e) => log::error!("Watcher error: {:?}", e),
    })?;

    watcher.watch(&path, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}
