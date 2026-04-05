use serde::{Deserialize, Serialize};

use tauri::Url;

#[derive(Deserialize)]
pub struct ConfigBase {
    pub url: Option<String>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub url: Option<String>,
    #[serde(skip_serializing)]
    path: String
}


impl Config {
  pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    println!("===============Config Loading=================");
    #[cfg(debug_assertions)]
    println!("Loading config from: {}", path);
    let content = Self::load_from_file(path)?;
    #[cfg(debug_assertions)]
    println!("Config content: {:?}", content);
    let mut config: ConfigBase = toml::from_str(&content)?;
    if config.url.is_some() {
      #[cfg(debug_assertions)]
      println!("Validating URL: {}", config.url.clone().unwrap());
      let url = Url::parse(&config.url.clone().unwrap());

      if url.is_err() {
        #[cfg(debug_assertions)]
        println!("Invalid URL, resetting to None");
        config.url = None;
      }
    }
    let config = Config {
      url: config.url,
      path: path.to_string(),
    };
    #[cfg(debug_assertions)]
    println!("Final Config: {:?}", config);
    #[cfg(debug_assertions)]
    println!("============================================");
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

  // fn reload_from_file(&self) -> Result<String, Box<dyn std::error::Error>> {
  //   Self::load_from_file(&self.path)
  // }

  fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
    let toml_string = toml::to_string(self)?;
    std::fs::write(&self.path, toml_string)?;
    Ok(())
  }

  pub fn update_url(&mut self, new_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    println!("=============URL Config Update==============");
    #[cfg(debug_assertions)]
    println!("Updating URL to: {}", new_url);
    let parsed_url = Url::parse(new_url);
    if parsed_url.is_err() {
      #[cfg(debug_assertions)]
      println!("Invalid URL format: {}", new_url);
      return Err("Invalid URL format".into());
    }
    self.url = Some(new_url.to_string());
    #[cfg(debug_assertions)]
    println!("Updated Config: {:?}", self);
    self.save_to_file()?;
    #[cfg(debug_assertions)]
    println!("Config saved to file: {}", self.path);
    #[cfg(debug_assertions)]
    println!("============================================");
    Ok(())
  }
}
