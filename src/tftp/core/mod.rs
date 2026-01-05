//! TFTP 核心协议实现
//!
//! 本模块包含 TFTP 协议的核心组件：
//! - `packet`: 协议包的序列化和反序列化
//! - `socket`: Socket 抽象层
//! - `options`: 协议选项和参数
//! - `window`: 窗口化传输管理
//! - `convert`: 数据转换工具

mod convert;
pub mod options;
mod packet;
mod socket;
mod window;

// 公开核心类型
pub use convert::Convert;
pub use options::{OptionType, TransferOption};
pub use packet::{ErrorCode, Packet};
pub use socket::{ServerSocket, Socket};
pub use window::Window;
