//! TFTP 服务器实现
//!
//! 本模块提供完整的 TFTP 服务器功能：
//! - `server`: 主服务器逻辑，处理客户端请求
//! - `worker`: 工作线程，处理文件传输
//! - `config`: 服务器配置

mod config;
mod server;
mod worker;

use anyhow::Result;
use std::path::PathBuf;

// 公开服务器类型
pub use config::Config;
pub use server::Server;
pub use worker::Worker;

/// 运行 TFTP 服务器
pub fn run(ip: String, port: u16, path: PathBuf, read_only: bool, single_port: bool) -> Result<()> {
    log::info!("Starting TFTP server on {}:{}", ip, port);
    log::info!("Root directory: {}", path.display());
    log::info!("Read-only mode: {}", read_only);
    log::info!("Single port mode: {}", single_port);

    // 确保目录存在
    if !path.exists() {
        log::error!("Directory does not exist: {}", path.display());
        return Err(anyhow::anyhow!("Directory does not exist"));
    }

    let ip_addr = ip
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid IP address '{}': {}", ip, e))?;

    let config = Config::new(ip_addr, port, path, read_only).with_single_port(single_port);

    let mut server = Server::new(&config)?;

    log::info!("TFTP server listening, press Ctrl+C to stop");
    server.listen();

    Ok(())
}
