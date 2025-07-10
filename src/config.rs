use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub repo_url: String,
    pub auth: AuthMethod,
    pub files_to_backup: Vec<PathBuf>,
    pub backup_schedule: String,
    pub commit_message_template: String,
    pub log_file: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AuthMethod {
    Ssh,
    Pat(String),
}

impl Config {
    pub fn load() -> Result<Self, std::io::Error> {
        let config_path = config_path()?;
        let config_str = std::fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_str).unwrap();
        Ok(config)
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_path = config_path()?;
        let config_str = serde_json::to_string_pretty(self).unwrap();
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        std::fs::write(config_path, config_str)
    }
}

fn config_path() -> Result<PathBuf, std::io::Error> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Config directory not found")
    })?;
    Ok(config_dir.join("giterdone").join("config.json"))
}
