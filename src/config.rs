use etcetera::{self, BaseStrategy};
use serde::Deserialize;
use std::{fs, io};
use thiserror::Error;

#[derive(Deserialize)]
pub struct Config {
    pub jira: JiraConfig,
}

#[derive(Deserialize)]
pub struct JiraConfig {
    pub host: String,
    pub email: String,
    pub api_token: String,
}

impl Config {
    pub fn from_file() -> Result<Config, ConfigError> {
        let mut config_file = etcetera::choose_base_strategy()
            .map(|x| x.config_dir())
            .unwrap();
        config_file.push("refrences-lsp/config.toml");
        let file_contents = fs::read_to_string(config_file)?;
        let config: Config = toml::from_str(&file_contents).unwrap();
        Ok(config)
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Error reading file")]
    FileReadError(#[from] io::Error),
    #[error("Error parsing file")]
    ParseError,
    #[error("Something else")]
    OtherError,
}
