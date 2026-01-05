//! TFTP (Trivial File Transfer Protocol) 实现
//!
//! 本模块实现了完整的 TFTP 协议，基于以下 RFC 标准：
//! - [RFC 1350](https://www.rfc-editor.org/rfc/rfc1350) TFTP 协议版本 2
//! - [RFC 2347](https://www.rfc-editor.org/rfc/rfc2347) TFTP 选项扩展
//! - [RFC 2348](https://www.rfc-editor.org/rfc/rfc2348) Blocksize 选项
//! - [RFC 2349](https://www.rfc-editor.org/rfc/rfc2349) Timeout 和 Transfer Size 选项
//! - [RFC 7440](https://www.rfc-editor.org/rfc/rfc7440) Windowsize 选项
//!
//! ## 模块结构
//!
//! ```text
//! tftp/
//! ├── core/           # 核心协议实现
//! │   ├── packet      # 协议包序列化/反序列化
//! │   ├── socket      # Socket 抽象层
//! │   ├── options     # 协议选项
//! │   ├── window      # 窗口化传输
//! │   └── convert     # 数据转换工具
//! │
//! ├── server/         # TFTP 服务器
//! │   ├── server      # 主服务器逻辑
//! │   ├── worker      # 传输工作线程
//! │   └── config      # 服务器配置
//! │
//! └── client/         # TFTP 客户端（未来）
//!     └── ...
//! ```
//!
//! ## 使用示例
//!
//! ### 启动 TFTP 服务器
//!
//! ```rust,no_run
//! use xtool::tftp::{server::Config, server::Server};
//! use std::path::PathBuf;
//!
//! let config = Config::new(
//!     "0.0.0.0".parse().unwrap(),
//!     69,
//!     PathBuf::from("/var/tftp"),
//!     false, // read_only
//! );
//!
//! let mut server = Server::new(&config).unwrap();
//! server.listen();
//! ```

// 子模块
pub mod client;
pub mod core;
pub mod server;

// 重新导出常用类型，方便使用
