// config.rs
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    pub url: String,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub news_sources: Vec<Source>,
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_str: String = fs::read_to_string("sources.json")?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}
