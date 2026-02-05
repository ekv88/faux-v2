use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use eframe::egui;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
  pub test: bool,
  pub main_position: Option<WindowPosition>,
  pub api_key: String,
  pub opacity: f32,
  pub stealth: bool,
  pub background: ColorConfig,
  pub text_color: ColorConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowPosition {
  pub x: f32,
  pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorConfig {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: u8,
}

impl ColorConfig {
  pub fn to_color32(self) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
  }

  pub fn from_color32(color: egui::Color32) -> Self {
    let [r, g, b, a] = color.to_array();
    Self { r, g, b, a }
  }
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      test: false,
      main_position: None,
      api_key: String::new(),
      opacity: 0.6,
      stealth: true,
      background: ColorConfig {
        r: 0x33,
        g: 0x33,
        b: 0x33,
        a: 0xFF,
      },
      text_color: ColorConfig {
        r: 210,
        g: 210,
        b: 210,
        a: 0xFF,
      },
    }
  }
}

pub fn current_dir_config_path() -> PathBuf {
  std::env::current_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config.json")
}

pub fn read_config(path: &Path) -> AppConfig {
  match fs::read_to_string(path) {
    Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
    Err(_) => {
      let config = AppConfig::default();
      let _ = write_config(path, &config);
      config
    }
  }
}

pub fn write_config(path: &Path, config: &AppConfig) -> Result<(), String> {
  let contents = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
  fs::write(path, contents).map_err(|e| e.to_string())
}
