use log::info;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::tftp::client::config::ClientConfig;
use crate::tftp::client::config::TftpcConfigFile;
use crate::tftp::server::config::Config as TftpdConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tftpd: Option<TftpdConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tftpc: Option<TftpcConfigFile>,
}

impl AppConfig {
    pub fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn generate_config_file(force: bool) -> anyhow::Result<()> {
        use std::io::Write;
        
        let config_path = ".xtool.toml";
        
        // Check if file already exists
        if std::path::Path::new(config_path).exists() && !force {
            anyhow::bail!("Configuration file {} already exists. Use --force to overwrite.", config_path);
        }
        
        // Generate configuration content
        let config_content = Self::generate_full_config();
        
        // Write to file
        let mut file = fs::File::create(config_path)?;
        file.write_all(config_content.as_bytes())?;
        
        info!("Configuration file generated: {}", config_path);
        info!("Contains full configuration (server + client)");
        info!("Please edit this file to customize configuration");
        Ok(())
    }

    pub fn generate_full_config() -> String {
        let config = AppConfig {
            tftpd: Some(TftpdConfig::with_defaults()),
            tftpc: Some(TftpcConfigFile {
                get: Some(ClientConfig::new("127.0.0.1".to_string(), 69)),
                put: Some(ClientConfig::new("127.0.0.1".to_string(), 69)),
            }),
        };
        let toml_content = toml::to_string_pretty(&config).unwrap();
        format!("# xtool configuration file\n# All fields are optional, command line arguments override config file values\n\n{}", toml_content)
    }
}
