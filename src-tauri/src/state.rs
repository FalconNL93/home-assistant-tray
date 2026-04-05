use serde::{Deserialize, Serialize};

use tauri::Url;

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub url: Option<String>,
    pub token: Option<String>,
    pub device_name: Option<String>,
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(skip)]
    path: String,
}

fn default_true() -> bool { true }


impl Config {
  pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    println!("[Config] loading from: {}", path);

    let content = Self::load_from_file(path)?;
    let mut config: Config = toml::from_str(&content)?;
    config.path = path.to_string();

    if let Some(ref url_str) = config.url.clone() {
      if Url::parse(url_str).is_err() {
        #[cfg(debug_assertions)]
        println!("[Config] invalid URL in config, resetting to None");
        config.url = None;
      }
    }

    #[cfg(debug_assertions)]
    println!("[Config] loaded: {:?}", config);
    Ok(config)
  }

  fn load_from_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    match std::fs::read_to_string(path) {
      Ok(content) => Ok(content),
      Err(_) => {
        if let Some(parent) = std::path::Path::new(path).parent() {
          std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, "")?;
        Ok(String::new())
      }
    }
  }

  fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
    let toml_string = toml::to_string(self)?;
    std::fs::write(&self.path, toml_string)?;
    Ok(())
  }

  pub fn update_url(&mut self, new_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if Url::parse(new_url).is_err() {
      return Err("Invalid URL format".into());
    }
    self.url = Some(new_url.to_string());
    self.save_to_file()
  }

  pub fn update_token(&mut self, new_token: &str) -> Result<(), Box<dyn std::error::Error>> {
    self.token = Some(new_token.to_string());
    self.save_to_file()?;
    Ok(())
  }

  pub fn update_prefs(
    &mut self,
    device_name: Option<String>,
    auto_update: bool,
    notifications_enabled: bool,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.device_name = device_name;
    self.auto_update = auto_update;
    self.notifications_enabled = notifications_enabled;
    self.save_to_file()
  }
}
