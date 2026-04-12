pub mod types;

use anyhow::{Context, Result};
use std::path::PathBuf;
use types::Config;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("loquitor")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn lanes_dir() -> PathBuf {
    config_dir().join("lanes")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config at {}", path.display()))?;
    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path();
    let dir = config_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create config directory at {}", dir.display()))?;
    let content = toml::to_string_pretty(config)
        .context("Failed to serialize config to TOML")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}
