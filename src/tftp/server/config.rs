use std::net::IpAddr;
use std::path::PathBuf;

use crate::tftp::core::options::OptionsPrivate;

/// TFTP 服务器配置
///
/// 提供简化的配置接口，适配 xtool 项目的需求
///
/// # 示例
///
/// ```rust
/// use xtool::tftp::server::Config;
/// use std::path::PathBuf;
///
/// let config = Config::new(
///     "127.0.0.1".parse().unwrap(),
///     69,
///     PathBuf::from("/tmp/tftp"),
///     false,
/// );
/// ```
pub struct Config {
    /// 监听的 IP 地址
    pub ip_address: IpAddr,
    /// 监听的端口号
    pub port: u16,
    /// 上传文件的目录（默认与 directory 相同）
    pub receive_directory: PathBuf,
    /// 下载文件的目录（默认与 directory 相同）
    pub send_directory: PathBuf,
    /// 是否使用单端口模式（用于NAT环境）
    pub single_port: bool,
    /// 是否为只读模式（拒绝所有写请求）
    pub read_only: bool,
    /// 是否覆盖已存在的文件
    pub overwrite: bool,
    /// 内部选项（重试次数、超时等）
    pub opt_local: OptionsPrivate,
}

impl Config {
    /// 创建一个新的配置
    ///
    /// # 参数
    ///
    /// * `ip_address` - 监听的 IP 地址
    /// * `port` - 监听的端口号
    /// * `directory` - 文件根目录
    /// * `read_only` - 是否为只读模式
    pub fn new(ip_address: IpAddr, port: u16, directory: PathBuf, read_only: bool) -> Self {
        let receive_directory = directory.clone();
        let send_directory = directory;

        Self {
            ip_address,
            port,
            receive_directory,
            send_directory,
            single_port: false,
            read_only,
            overwrite: true, // 默认允许覆盖
            opt_local: OptionsPrivate::default(),
        }
    }

    /// 设置是否使用单端口模式
    pub fn with_single_port(mut self, single_port: bool) -> Self {
        self.single_port = single_port;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        use std::net::Ipv4Addr;

        Self::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            69,
            std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir()),
            false,
        )
    }
}
