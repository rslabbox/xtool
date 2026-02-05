//! TFTP client implementation
//!
//! This module provides TFTP client functionality:
//! - File download (GET/RRQ)
//! - File upload (PUT/WRQ)
//! - Supports all TFTP option extensions
//!
//! # Usage Examples
//!
//! ## Download file
//!
//! ```rust,no_run
//! use xtool::tftp::client::Client;
//! use xtool::tftp::client::config::ClientConfig;
//! use std::path::Path;
//!
//! let config = ClientConfig::new("192.168.1.100".to_string(), 69);
//! let client = Client::new(config).unwrap();
//! client.get("remote.txt", Path::new("local.txt")).unwrap();
//! ```
//!
//! ## Upload file
//!
//! ```rust,no_run
//! use xtool::tftp::client::Client;
//! use xtool::tftp::client::config::ClientConfig;
//! use std::path::Path;
//!
//! let config = ClientConfig::new("192.168.1.100".to_string(), 69);
//! let client = Client::new(config).unwrap();
//! client.put(Path::new("local.txt"), "remote.txt").unwrap();
//! ```
//!
//! # Command Line Usage
//!
//! ```bash
//! # Download file
//! xtool tftpc get 192.168.1.100 remote.txt [local.txt]
//!
//! # Upload file
//! xtool tftpc put 192.168.1.100 local.txt [remote.txt]
//! ```

mod client_impl;
pub mod config;

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

pub use client_impl::Client;

#[derive(Subcommand)]
pub enum TftpcAction {
    /// Download a file from TFTP server (RRQ)
    Get {
        /// Server IP address or hostname
        server: String,

        /// Remote file name on server
        remote_file: String,

        /// Local file path (defaults to remote file name)
        #[arg(value_name = "LOCAL_FILE")]
        local_file: Option<PathBuf>,

        /// Server port
        #[arg(short, long, default_value = "69")]
        port: u16,

        /// Block size (512-65464)
        #[arg(short, long, default_value = "512")]
        block_size: u16,

        /// Timeout in seconds
        #[arg(short, long, default_value = "5")]
        timeout: u64,
    },

    /// Upload a file to TFTP server (WRQ)
    Put {
        /// Server IP address or hostname
        server: String,

        /// Local file path to upload
        local_file: PathBuf,

        /// Remote file name on server (defaults to local file name)
        #[arg(value_name = "REMOTE_FILE")]
        remote_file: Option<String>,

        /// Server port
        #[arg(short, long, default_value = "69")]
        port: u16,

        /// Block size (512-65464)
        #[arg(short, long, default_value = "512")]
        block_size: u16,

        /// Timeout in seconds
        #[arg(short, long, default_value = "5")]
        timeout: u64,
    },
}

/// Run TFTP client command with configuration
pub fn run_with_config(
    action: TftpcAction,
    config: Option<&config::TftpcConfigFile>,
) -> Result<()> {
    match action {
        TftpcAction::Get {
            server,
            remote_file,
            local_file,
            port,
            block_size,
            timeout,
        } => {
            let client_config = config.and_then(|c| c.get.clone()).unwrap_or_default();
            let cfg = client_config.merge_cli(server.clone(), port, block_size, timeout);

            let local_path = local_file.unwrap_or_else(|| PathBuf::from(&remote_file));

            // Note: cfg.server is Option<String>, but merge_cli ensures it's set if cli_server is provided
            let server_display = cfg.server.as_deref().unwrap_or("unknown");
            let port_display = cfg.port.unwrap_or(69);

            log::info!(
                "Downloading {} from {}:{}",
                remote_file,
                server_display,
                port_display
            );
            log::info!("Saving to: {}", local_path.display());

            let client = Client::new(cfg)?;
            client.get(&remote_file, &local_path)?;

            log::info!("Download completed successfully");
        }

        TftpcAction::Put {
            server,
            local_file,
            remote_file,
            port,
            block_size,
            timeout,
        } => {
            let client_config = config.and_then(|c| c.put.clone()).unwrap_or_default();
            let cfg = client_config.merge_cli(server.clone(), port, block_size, timeout);

            if !local_file.exists() {
                log::error!("Local file does not exist: {}", local_file.display());
                return Err(anyhow::anyhow!("Local file does not exist"));
            }

            let remote_name = remote_file.unwrap_or_else(|| {
                local_file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string()
            });

            let server_display = cfg.server.as_deref().unwrap_or("unknown");
            let port_display = cfg.port.unwrap_or(69);

            log::info!(
                "Uploading {} to {}:{}",
                local_file.display(),
                server_display,
                port_display
            );
            log::info!("Remote file: {}", remote_name);

            let client = Client::new(cfg)?;
            client.put(&local_file, &remote_name)?;

            log::info!("Upload completed successfully");
        }
    }
    Ok(())
}
