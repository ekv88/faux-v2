use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use eframe::egui;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
  pub test: bool,
  pub main_position: Option<WindowPosition>,
  pub api_key: String,
  pub opacity: f32,
  pub stealth: bool,
  pub always_on_top: bool,
  pub background: ColorConfig,
  pub text_color: ColorConfig,
  pub divider_color: ColorConfig,
  pub response_max_width: f32,
  pub response_max_height: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowPosition {
  pub x: f32,
  pub y: f32,
}

#[derive(Debug, Clone, Copy)]
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

impl Serialize for ColorConfig {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let value = format!("{}, {}, {}, {}", self.r, self.g, self.b, self.a);
    serializer.serialize_str(&value)
  }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ColorConfigRepr {
  Str(String),
  Obj {
    r: u8,
    g: u8,
    b: u8,
    #[serde(default)]
    a: Option<u8>,
  },
}

impl<'de> Deserialize<'de> for ColorConfig {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let parsed = ColorConfigRepr::deserialize(deserializer)?;
    match parsed {
      ColorConfigRepr::Obj { r, g, b, a } => Ok(Self {
        r,
        g,
        b,
        a: a.unwrap_or(255),
      }),
      ColorConfigRepr::Str(value) => {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        let mut nums: Vec<u8> = Vec::with_capacity(parts.len());
        for part in parts {
          if part.is_empty() {
            continue;
          }
          if let Ok(num) = part.parse::<u8>() {
            nums.push(num);
          } else {
            return Ok(Self { r: 0, g: 0, b: 0, a: 255 });
          }
        }
        if nums.len() == 3 {
          return Ok(Self { r: nums[0], g: nums[1], b: nums[2], a: 255 });
        }
        if nums.len() >= 4 {
          return Ok(Self { r: nums[0], g: nums[1], b: nums[2], a: nums[3] });
        }
        Ok(Self { r: 0, g: 0, b: 0, a: 255 })
      }
    }
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
      always_on_top: true,
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
      divider_color: ColorConfig {
        r: 90,
        g: 90,
        b: 90,
        a: 0xFF,
      },
      response_max_width: 860.0,
      response_max_height: 620.0,
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
