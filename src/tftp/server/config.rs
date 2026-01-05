use crate::tftp::core::options::{OptionsPrivate, Rollover};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// TFTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receive_directory: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_directory: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_port: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwrite: Option<bool>,

    // OptionsPrivate fields flattened
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_count: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clean_on_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollover: Option<Rollover>,
}

impl Config {
    pub fn with_defaults() -> Self {
        Self {
            ip: Some("0.0.0.0".to_string()),
            port: Some(69),
            directory: None,
            receive_directory: None,
            send_directory: None,
            single_port: Some(false),
            read_only: Some(false),
            overwrite: Some(true),
            repeat_count: Some(1),
            clean_on_error: Some(true),
            max_retries: Some(6),
            rollover: Some(Rollover::Enforce0),
        }
    }

    pub fn merge_cli(
        mut self,
        cli_ip: String,
        cli_port: u16,
        cli_path: PathBuf,
        cli_read_only: bool,
        cli_single_port: bool,
    ) -> Self {
        if self.ip.is_none() {
            self.ip = Some(cli_ip);
        }
        if self.port.is_none() {
            self.port = Some(cli_port);
        }
        if self.directory.is_none() {
            self.directory = Some(cli_path);
        }
        if self.read_only.is_none() {
            self.read_only = Some(cli_read_only);
        }
        if self.single_port.is_none() {
            self.single_port = Some(cli_single_port);
        }

        // Set defaults for others if not present
        if self.overwrite.is_none() {
            self.overwrite = Some(true);
        }
        if self.repeat_count.is_none() {
            self.repeat_count = Some(1);
        }
        if self.clean_on_error.is_none() {
            self.clean_on_error = Some(true);
        }
        if self.max_retries.is_none() {
            self.max_retries = Some(6);
        }
        if self.rollover.is_none() {
            self.rollover = Some(Rollover::Enforce0);
        }

        self
    }

    pub fn get_options(&self) -> OptionsPrivate {
        OptionsPrivate {
            repeat_count: self.repeat_count.unwrap_or(1),
            clean_on_error: self.clean_on_error.unwrap_or(true),
            max_retries: self.max_retries.unwrap_or(6),
            rollover: self.rollover.unwrap_or(Rollover::Enforce0),
        }
    }
}
