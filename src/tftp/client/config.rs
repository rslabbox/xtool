use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TftpcConfigFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub get: Option<ClientConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put: Option<ClientConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_size: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none", with = "humantime_serde")]
    pub timeout: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_size: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl ClientConfig {
    pub fn new(server: String, port: u16) -> Self {
        Self {
            server: Some(server),
            port: Some(port),
            block_size: Some(512),
            timeout: Some(Duration::from_secs(5)),
            window_size: Some(1),
            mode: Some("octet".to_string()),
        }
    }

    pub fn merge_cli(
        mut self,
        cli_server: String,
        cli_port: u16,
        cli_block_size: u16,
        cli_timeout: u64,
    ) -> Self {
        // CLI args are used if config file doesn't specify them
        // (Matching previous behavior: File > CLI)
        if self.server.is_none() {
            self.server = Some(cli_server);
        }
        if self.port.is_none() {
            self.port = Some(cli_port);
        }
        if self.block_size.is_none() {
            self.block_size = Some(cli_block_size);
        }
        if self.timeout.is_none() {
            self.timeout = Some(Duration::from_secs(cli_timeout));
        }
        if self.window_size.is_none() {
            self.window_size = Some(1);
        }
        if self.mode.is_none() {
            self.mode = Some("octet".to_string());
        }
        self
    }

    #[allow(dead_code)]
    pub fn with_block_size(mut self, block_size: u16) -> Self {
        self.block_size = Some(block_size);
        self
    }

    #[allow(dead_code)]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    #[allow(dead_code)]
    pub fn with_window_size(mut self, window_size: u16) -> Self {
        self.window_size = Some(window_size);
        self
    }
}
