//! TFTP server implementation
//!
//! This module provides complete TFTP server functionality:
//! - `server`: Main server logic, handles client requests
//! - `worker`: Worker threads, handles file transfers
//! - `config`: Server configuration

pub mod config;
mod server;
mod worker;

use anyhow::Result;
use std::path::PathBuf;

// Public server types
pub use config::Config;
pub use server::Server;
pub use worker::Worker;

/// Run the TFTP server with CLI arguments and optional configuration
pub fn run_with_config(
    ip: String,
    port: u16,
    path: PathBuf,
    read_only: bool,
    single_port: bool,
    config: Option<Config>,
) -> Result<()> {
    let server_config = config.unwrap_or_default();
    let config = server_config.merge_cli(ip, port, path, read_only, single_port);

    let ip = config.ip.as_deref().unwrap_or("0.0.0.0");
    let port = config.port.unwrap_or(69);
    let directory = config
        .directory
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    let receive_directory = config
        .receive_directory
        .clone()
        .unwrap_or_else(|| directory.clone());
    let send_directory = config
        .send_directory
        .clone()
        .unwrap_or_else(|| directory.clone());
    let read_only = config.read_only.unwrap_or(false);
    let single_port = config.single_port.unwrap_or(false);

    log::info!("Starting TFTP server on {}:{}", ip, port);
    log::info!("Receive directory: {}", receive_directory.display());
    log::info!("Send directory: {}", send_directory.display());
    log::info!("Read-only mode: {}", read_only);
    log::info!("Single port mode: {}", single_port);

    // Ensure directories exist
    if !receive_directory.exists() {
        log::error!(
            "Receive directory does not exist: {}",
            receive_directory.display()
        );
        return Err(anyhow::anyhow!("Receive directory does not exist"));
    }
    if !send_directory.exists() {
        log::error!(
            "Send directory does not exist: {}",
            send_directory.display()
        );
        return Err(anyhow::anyhow!("Send directory does not exist"));
    }

    let mut server = Server::new(&config)?;

    log::info!("TFTP server listening, press Ctrl+C to stop");
    server.listen();

    Ok(())
}
