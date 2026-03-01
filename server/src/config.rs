use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{info, warn};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub org_path: Vec<String>,
    pub server_port: u16,
    #[serde(default = "default_server_host")]
    pub server_host: String,
    #[serde(default = "default_mcp_host")]
    pub mcp_host: String,
    #[serde(default = "default_mcp_port")]
    pub mcp_port: u16,
}

pub fn parse_config(path: &Path) -> Result<Config> {
    let config_toml =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut config: Config = toml::from_str(&config_toml)
        .with_context(|| format!("failed to parse config {}", path.display()))?;
    config.org_path = config
        .org_path
        .iter()
        .map(|p| expand_org_path(p))
        .collect::<Result<Vec<_>>>()?;
    info!("load config {:?}", config);
    Ok(config)
}

fn expand_org_path(path: &str) -> Result<String> {
    shellexpand::full(path)
        .map(|expanded| expanded.into_owned())
        .map_err(|err| anyhow::anyhow!("failed to expand org_path '{}': {}", path, err))
}

fn escape_toml_basic_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn write_default_config(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;
    }

    let org_dir = default_org_dir();
    let escaped_org_dir = escape_toml_basic_string(&org_dir.display().to_string());
    let template = format!(
        "org_path = [\"{}\"]\nserver_port = 3000\nserver_host = \"127.0.0.1\"\nmcp_host = \"127.0.0.1\"\nmcp_port = 3001\n",
        escaped_org_dir
    );
    fs::write(path, template)
        .with_context(|| format!("failed to write default config {}", path.display()))?;
    warn!(
        "Config file not found. Generated default config at {}",
        path.display()
    );
    Ok(())
}

fn default_org_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join("org")
    } else {
        PathBuf::from("./org")
    }
}

fn default_mcp_host() -> String {
    "127.0.0.1".to_string()
}

fn default_server_host() -> String {
    "127.0.0.1".to_string()
}

const fn default_mcp_port() -> u16 {
    3001
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_config_expands_tilde() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("org-server.toml");
        fs::write(
            &config_path,
            "org_path = [\"~/org\"]\nserver_port = 3000\nserver_host = \"127.0.0.1\"\n",
        )?;

        let config = parse_config(&config_path)?;
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("home dir is unavailable"))?;
        assert_eq!(config.org_path[0], home.join("org").display().to_string());
        Ok(())
    }

    #[test]
    fn test_write_default_config_creates_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("org-server.toml");

        write_default_config(&config_path)?;
        assert!(config_path.exists());

        let loaded = parse_config(&config_path)?;
        assert_eq!(loaded.server_port, 3000);
        assert_eq!(loaded.server_host, "127.0.0.1");
        assert_eq!(loaded.mcp_host, "127.0.0.1");
        assert_eq!(loaded.mcp_port, 3001);
        assert!(!loaded.org_path.is_empty());
        Ok(())
    }

    #[test]
    fn test_write_default_config_does_not_overwrite_existing() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("org-server.toml");
        fs::write(
            &config_path,
            "org_path = [\"/tmp/custom\"]\nserver_port = 7777\nserver_host = \"0.0.0.0\"\n",
        )?;

        write_default_config(&config_path)?;

        let loaded = parse_config(&config_path)?;
        assert_eq!(loaded.server_port, 7777);
        assert_eq!(loaded.server_host, "0.0.0.0");
        assert_eq!(loaded.org_path, vec!["/tmp/custom".to_string()]);
        Ok(())
    }
}
