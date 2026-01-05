//! TFTP 客户端实现
//!
//! 本模块提供 TFTP 客户端功能：
//! - 文件下载（GET/RRQ）
//! - 文件上传（PUT/WRQ）
//! - 支持所有 TFTP 选项扩展
//!
//! # 使用示例
//!
//! ## 下载文件
//!
//! ```rust,no_run
//! use xtool::tftp::client::{Client, ClientConfig};
//! use std::path::Path;
//!
//! let config = ClientConfig::new("192.168.1.100".parse().unwrap(), 69);
//! let client = Client::new(config).unwrap();
//! client.get("remote.txt", Path::new("local.txt")).unwrap();
//! ```
//!
//! ## 上传文件
//!
//! ```rust,no_run
//! use xtool::tftp::client::{Client, ClientConfig};
//! use std::path::Path;
//!
//! let config = ClientConfig::new("192.168.1.100".parse().unwrap(), 69);
//! let client = Client::new(config).unwrap();
//! client.put(Path::new("local.txt"), "remote.txt").unwrap();
//! ```
//!
//! # 命令行使用
//!
//! ```bash
//! # 下载文件
//! xtool tftpc get 192.168.1.100 remote.txt [local.txt]
//!
//! # 上传文件
//! xtool tftpc put 192.168.1.100 local.txt [remote.txt]
//! ```

mod client;
mod config;

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

pub use client::Client;
pub use config::ClientConfig;

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

/// 运行 TFTP 客户端命令
pub fn run(action: TftpcAction) -> Result<()> {
    match action {
        TftpcAction::Get {
            server,
            remote_file,
            local_file,
            port,
            block_size,
            timeout,
        } => {
            run_get(server, remote_file, local_file, port, block_size, timeout)?;
        }

        TftpcAction::Put {
            server,
            local_file,
            remote_file,
            port,
            block_size,
            timeout,
        } => {
            run_put(server, local_file, remote_file, port, block_size, timeout)?;
        }
    }
    Ok(())
}

/// 运行 TFTP 客户端下载命令
pub fn run_get(
    server: String,
    remote_file: String,
    local_file: Option<PathBuf>,
    port: u16,
    block_size: u16,
    timeout: u64,
) -> Result<()> {
    let server_ip = server
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid server address '{}': {}", server, e))?;

    let local_path = local_file.unwrap_or_else(|| PathBuf::from(&remote_file));

    log::info!("Downloading {} from {}:{}", remote_file, server, port);
    log::info!("Saving to: {}", local_path.display());

    let config = ClientConfig::new(server_ip, port)
        .with_block_size(block_size)
        .with_timeout(std::time::Duration::from_secs(timeout));

    let client = Client::new(config)?;
    client.get(&remote_file, &local_path)?;

    log::info!("Download completed successfully");
    Ok(())
}

/// 运行 TFTP 客户端上传命令
pub fn run_put(
    server: String,
    local_file: PathBuf,
    remote_file: Option<String>,
    port: u16,
    block_size: u16,
    timeout: u64,
) -> Result<()> {
    if !local_file.exists() {
        log::error!("Local file does not exist: {}", local_file.display());
        return Err(anyhow::anyhow!("Local file does not exist"));
    }

    let server_ip = server
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid server address '{}': {}", server, e))?;

    let remote_name = remote_file.unwrap_or_else(|| {
        local_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string()
    });

    log::info!("Uploading {} to {}:{}", local_file.display(), server, port);
    log::info!("Remote file: {}", remote_name);

    let config = ClientConfig::new(server_ip, port)
        .with_block_size(block_size)
        .with_timeout(std::time::Duration::from_secs(timeout));

    let client = Client::new(config)?;
    client.put(&local_file, &remote_name)?;

    log::info!("Upload completed successfully");
    Ok(())
}
