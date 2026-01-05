use std::fs::File;
use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::path::Path;

use super::config::ClientConfig;
use crate::tftp::core::options::{OptionsProtocol, RequestType};
use crate::tftp::core::{Packet, TransferOption, Window};

/// TFTP 客户端
///
/// 支持文件的上传（PUT）和下载（GET）操作
///
/// # 示例
///
/// ```rust,no_run
/// use xtool::tftp::client::{Client, ClientConfig};
/// use std::path::Path;
///
/// let config = ClientConfig::new("192.168.1.100".parse().unwrap(), 69);
/// let client = Client::new(config).unwrap();
///
/// // 下载文件
/// client.get("remote.txt", Path::new("local.txt")).unwrap();
///
/// // 上传文件
/// client.put(Path::new("local.txt"), "remote.txt").unwrap();
/// ```
pub struct Client {
    config: ClientConfig,
}

impl Client {
    /// 创建新的 TFTP 客户端
    pub fn new(config: ClientConfig) -> anyhow::Result<Self> {
        Ok(Self { config })
    }

    /// 从服务器下载文件（RRQ - Read Request）
    ///
    /// # 参数
    ///
    /// * `remote_file` - 服务器上的文件名
    /// * `local_file` - 本地保存路径
    pub fn get(&self, remote_file: &str, local_file: &Path) -> anyhow::Result<()> {
        log::info!("Downloading {} to {}", remote_file, local_file.display());

        // 创建本地 socket
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let server_addr = SocketAddr::new(self.config.server_ip, self.config.server_port);
        // 不使用 connect，而是使用 send_to
        socket.set_read_timeout(Some(self.config.timeout))?;
        socket.set_write_timeout(Some(self.config.timeout))?;

        // 准备选项
        let mut options = vec![
            TransferOption {
                option: crate::tftp::core::OptionType::BlockSize,
                value: self.config.block_size as u64,
            },
            TransferOption {
                option: crate::tftp::core::OptionType::WindowSize,
                value: self.config.window_size as u64,
            },
            TransferOption {
                option: crate::tftp::core::OptionType::Timeout,
                value: self.config.timeout.as_secs(),
            },
            TransferOption {
                option: crate::tftp::core::OptionType::TransferSize,
                value: 0, // 请求服务器告知文件大小
            },
        ];

        // 发送 RRQ
        let rrq = Packet::Rrq {
            filename: remote_file.to_string(),
            mode: self.config.mode.clone(),
            options: options.clone(),
        };
        socket.send_to(&rrq.serialize()?, &server_addr)?;

        // 等待响应（OACK 或第一个数据包）
        let mut buf = vec![0u8; 65536];
        let (amt, new_addr) = socket.recv_from(&mut buf)?;
        let response = Packet::deserialize(&buf[..amt])?;

        // 重新连接到服务器的新端口（TFTP 服务器为每个传输创建新端口）
        if new_addr != server_addr {
            socket.connect(new_addr)?;
        } else {
            socket.connect(server_addr)?;
        }

        let worker_options = match response {
            Packet::Oack(ref opts) => {
                options = opts.clone();
                let opts = OptionsProtocol::parse(&mut options, RequestType::Read(0))?;

                // 发送 ACK 0 确认选项
                let ack = Packet::Ack(0);
                socket.send(&ack.serialize()?)?;

                opts
            }
            Packet::Data { .. } => OptionsProtocol::default(),
            Packet::Error { code, msg } => {
                return Err(anyhow::anyhow!("Server error {}: {}", code, msg));
            }
            _ => {
                return Err(anyhow::anyhow!("Unexpected packet type"));
            }
        };

        // 接收文件
        let file = File::create(local_file)?;

        // 如果收到的是 OACK，则等待第一个 DATA 包；否则第一个包就是 DATA
        let first_data_packet = if matches!(response, Packet::Oack(_)) {
            let (amt, _) = socket.recv_from(&mut buf)?;
            Packet::deserialize(&buf[..amt])?
        } else {
            response
        };

        self.receive_file(socket, file, worker_options, first_data_packet)?;

        log::info!("Download complete: {}", local_file.display());
        Ok(())
    }

    /// 上传文件到服务器（WRQ - Write Request）
    ///
    /// # 参数
    ///
    /// * `local_file` - 本地文件路径
    /// * `remote_file` - 服务器上的文件名
    pub fn put(&self, local_file: &Path, remote_file: &str) -> anyhow::Result<()> {
        log::info!("Uploading {} to {}", local_file.display(), remote_file);

        if !local_file.exists() {
            return Err(anyhow::anyhow!("Local file does not exist"));
        }

        let file_size = local_file.metadata()?.len();

        // 创建本地 socket
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let server_addr = SocketAddr::new(self.config.server_ip, self.config.server_port);
        // 不使用 connect，而是使用 send_to
        socket.set_read_timeout(Some(self.config.timeout))?;
        socket.set_write_timeout(Some(self.config.timeout))?;

        // 准备选项
        let mut options = vec![
            TransferOption {
                option: crate::tftp::core::OptionType::BlockSize,
                value: self.config.block_size as u64,
            },
            TransferOption {
                option: crate::tftp::core::OptionType::WindowSize,
                value: self.config.window_size as u64,
            },
            TransferOption {
                option: crate::tftp::core::OptionType::Timeout,
                value: self.config.timeout.as_secs(),
            },
            TransferOption {
                option: crate::tftp::core::OptionType::TransferSize,
                value: file_size,
            },
        ];

        // 发送 WRQ
        let wrq = Packet::Wrq {
            filename: remote_file.to_string(),
            mode: self.config.mode.clone(),
            options: options.clone(),
        };
        socket.send_to(&wrq.serialize()?, &server_addr)?;

        // 等待响应（OACK 或 ACK 0）
        let mut buf = vec![0u8; 65536];
        let (amt, new_addr) = socket.recv_from(&mut buf)?;
        let response = Packet::deserialize(&buf[..amt])?;

        // 重新连接到服务器的新端口（TFTP 服务器为每个传输创建新端口）
        if new_addr != server_addr {
            socket.connect(new_addr)?;
        } else {
            socket.connect(server_addr)?;
        }

        let worker_options = match response {
            Packet::Oack(ref opts) => {
                options = opts.clone();
                OptionsProtocol::parse(&mut options, RequestType::Write)?
            }
            Packet::Ack(0) => OptionsProtocol::default(),
            Packet::Error { code, msg } => {
                return Err(anyhow::anyhow!("Server error {}: {}", code, msg));
            }
            _ => {
                return Err(anyhow::anyhow!("Unexpected packet type"));
            }
        };

        // 发送文件
        let file = File::open(local_file)?;
        self.send_file(socket, file, worker_options)?;

        log::info!("Upload complete: {}", remote_file);
        Ok(())
    }

    /// 接收文件数据
    fn receive_file(
        &self,
        socket: UdpSocket,
        mut file: File,
        options: OptionsProtocol,
        first_packet: Packet,
    ) -> anyhow::Result<()> {
        let mut expected_block: u16 = 1;
        let mut total_bytes = 0u64;

        // 处理第一个包（如果是 DATA）
        if let Packet::Data { block_num, data } = first_packet {
            if block_num == 1 {
                file.write_all(&data)?;
                total_bytes += data.len() as u64;

                // 发送 ACK
                let ack = Packet::Ack(block_num);
                socket.send(&ack.serialize()?)?;

                expected_block = 2;

                // 如果数据小于块大小，传输完成
                if data.len() < options.block_size as usize {
                    log::debug!("Transfer complete. Total bytes: {}", total_bytes);
                    return Ok(());
                }
            }
        }

        // 继续接收后续数据包
        let mut buf = vec![0u8; 65536];
        loop {
            let (amt, _) = socket.recv_from(&mut buf)?;
            let packet = Packet::deserialize(&buf[..amt])?;

            match packet {
                Packet::Data { block_num, data } => {
                    if block_num == expected_block {
                        file.write_all(&data)?;
                        total_bytes += data.len() as u64;

                        // 发送 ACK
                        let ack = Packet::Ack(block_num);
                        socket.send(&ack.serialize()?)?;

                        // 如果数据小于块大小，传输完成
                        if data.len() < options.block_size as usize {
                            log::debug!("Transfer complete. Total bytes: {}", total_bytes);
                            break;
                        }

                        expected_block = expected_block.wrapping_add(1);
                    } else {
                        log::warn!(
                            "Received unexpected block {}, expected {}",
                            block_num,
                            expected_block
                        );
                        // 重新发送上一个 ACK
                        let ack = Packet::Ack(expected_block.wrapping_sub(1));
                        socket.send(&ack.serialize()?)?;
                    }
                }
                Packet::Error { code, msg } => {
                    return Err(anyhow::anyhow!("Server error {}: {}", code, msg));
                }
                _ => {
                    log::warn!("Received unexpected packet type");
                }
            }
        }

        Ok(())
    }

    /// 发送文件数据
    fn send_file(
        &self,
        socket: UdpSocket,
        file: File,
        options: OptionsProtocol,
    ) -> anyhow::Result<()> {
        let mut window = Window::new(options.window_size, options.block_size, file);
        let mut block_num: u16 = 1;
        let mut total_bytes = 0u64;

        loop {
            // 填充窗口
            let more = window.fill()?;

            // 发送窗口中的所有数据包
            for data in window.get_elements() {
                let packet = Packet::Data {
                    block_num,
                    data: data.clone(),
                };
                socket.send(&packet.serialize()?)?;
                total_bytes += data.len() as u64;
                block_num = block_num.wrapping_add(1);
            }

            // 如果没有更多数据，等待最后的 ACK 并退出
            if !more && window.get_elements().is_empty() {
                break;
            }

            // 等待 ACK
            let mut buf = vec![0u8; 65536];
            let (amt, _) = socket.recv_from(&mut buf)?;
            let packet = Packet::deserialize(&buf[..amt])?;

            match packet {
                Packet::Ack(ack_block) => {
                    log::debug!("Received ACK for block {}", ack_block);
                    // 清空窗口，准备下一批数据
                    window.clear();
                }
                Packet::Error { code, msg } => {
                    return Err(anyhow::anyhow!("Server error {}: {}", code, msg));
                }
                _ => {
                    log::warn!("Received unexpected packet type");
                }
            }

            if !more {
                break;
            }
        }

        log::debug!("Transfer complete. Total bytes: {}", total_bytes);
        Ok(())
    }
}
