use anyhow::Result;
use serde::Deserialize;
use std::{fs::File, io::Read};
use tracing::info;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub org_path: Vec<String>,
    pub server_port: u32,
    #[serde(default = "default_mcp_host")]
    pub mcp_host: String,
    #[serde(default = "default_mcp_port")]
    pub mcp_port: u16,
}

pub fn parse_config(path: &str) -> Result<Config> {
    let mut config_toml = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut config_toml)?;
    let config: Config = toml::from_str(&config_toml).expect("toml parse error");
    info!("load config {:?}", config);
    Ok(config)
}

fn default_mcp_host() -> String {
    "127.0.0.1".to_string()
}

const fn default_mcp_port() -> u16 {
    3001
}
