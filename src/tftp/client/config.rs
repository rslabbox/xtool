use std::net::IpAddr;
use std::time::Duration;

/// TFTP 客户端配置
///
/// # 示例
///
/// ```rust
/// use xtool::tftp::client::ClientConfig;
///
/// let config = ClientConfig::new("192.168.1.100".parse().unwrap(), 69);
/// ```
pub struct ClientConfig {
    /// 服务器 IP 地址
    pub server_ip: IpAddr,
    /// 服务器端口号
    pub server_port: u16,
    /// 块大小（默认 512，可协商）
    pub block_size: u16,
    /// 超时时间
    pub timeout: Duration,
    /// 窗口大小（RFC 7440）
    pub window_size: u16,
    /// 传输模式（目前只支持 octet）
    pub mode: String,
}

impl ClientConfig {
    /// 创建新的客户端配置
    ///
    /// # 参数
    ///
    /// * `server_ip` - 服务器 IP 地址
    /// * `server_port` - 服务器端口号（通常为 69）
    pub fn new(server_ip: IpAddr, server_port: u16) -> Self {
        Self {
            server_ip,
            server_port,
            block_size: 512,
            timeout: Duration::from_secs(5),
            window_size: 1,
            mode: "octet".to_string(),
        }
    }

    /// 设置块大小
    pub fn with_block_size(mut self, block_size: u16) -> Self {
        self.block_size = block_size;
        self
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new("127.0.0.1".parse().unwrap(), 69)
    }
}
