use std::fs::File;
use std::io::{Read, Seek, Write};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::path::Path;
use std::time::Duration;

use super::config::ClientConfig;
use crate::tftp::core::{OptionType, Packet, TransferOption};

/// TFTP client
///
/// Supports file upload (PUT) and download (GET) operations
pub struct Client {
    server_ip: IpAddr,
    server_port: u16,
    block_size: u16,
    timeout: Duration,
    window_size: u16,
    mode: String,
}

impl Client {
    /// Create a new TFTP client
    pub fn new(config: ClientConfig) -> anyhow::Result<Self> {
        let server_str = config
            .server
            .ok_or_else(|| anyhow::anyhow!("Server address not specified"))?;
        let server_ip: IpAddr = server_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid server address '{}': {}", server_str, e))?;

        Ok(Self {
            server_ip,
            server_port: config.port.unwrap_or(69),
            block_size: config.block_size.unwrap_or(512),
            timeout: config.timeout.unwrap_or(Duration::from_secs(5)),
            window_size: config.window_size.unwrap_or(1),
            mode: config.mode.unwrap_or_else(|| "octet".to_string()),
        })
    }

    fn build_options(&self, transfer_size: u64) -> Vec<TransferOption> {
        let mut options = Vec::new();

        options.push(TransferOption {
            option: OptionType::BlockSize,
            value: self.block_size as u64,
        });

        options.push(TransferOption {
            option: OptionType::Timeout,
            value: self.timeout.as_secs(),
        });

        options.push(TransferOption {
            option: OptionType::WindowSize,
            value: self.window_size as u64,
        });

        if transfer_size > 0 {
            options.push(TransferOption {
                option: OptionType::TransferSize,
                value: transfer_size,
            });
        }

        options
    }

    /// Download a file from the server (RRQ - Read Request)
    pub fn get(&self, remote_file: &str, local_file: &Path) -> anyhow::Result<()> {
        log::info!("Downloading {} to {}", remote_file, local_file.display());

        // Create local socket
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let mut server_addr = SocketAddr::new(self.server_ip, self.server_port);
        let mut tid_set = false;

        socket.set_read_timeout(Some(self.timeout))?;
        socket.set_write_timeout(Some(self.timeout))?;

        // Build options
        let options = self.build_options(0);

        // Send RRQ
        let rrq = Packet::Rrq {
            filename: remote_file.to_string(),
            mode: self.mode.clone(),
            options,
        };
        let bytes = rrq.serialize()?;
        socket.send_to(&bytes, server_addr)?;

        // Receive file
        let mut file = File::create(local_file)?;
        let mut block_num: u16 = 1;
        let mut retries = 0;
        let max_retries = 5;

        loop {
            let mut buf = vec![0; self.block_size as usize + 4];
            match socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    if !tid_set {
                        if src.ip() == self.server_ip {
                            server_addr = src;
                            tid_set = true;
                        } else {
                            continue;
                        }
                    } else if src != server_addr {
                        continue;
                    }

                    let packet = Packet::deserialize(&buf[..amt])?;
                    match packet {
                        Packet::Data {
                            block_num: block,
                            data,
                        } => {
                            if block == block_num {
                                file.write_all(&data)?;

                                // Send ACK
                                let ack = Packet::Ack(block);
                                socket.send_to(&ack.serialize()?, server_addr)?;

                                block_num = block_num.wrapping_add(1);
                                retries = 0;

                                if data.len() < self.block_size as usize {
                                    break; // End of file
                                }
                            }
                        }
                        Packet::Error { code, msg } => {
                            return Err(anyhow::anyhow!("TFTP Error {:?}: {}", code, msg));
                        }
                        Packet::Oack(_) => {
                            // Handle option negotiation
                            if block_num == 1 {
                                // Send ACK 0 to confirm options
                                let ack = Packet::Ack(0);
                                socket.send_to(&ack.serialize()?, server_addr)?;
                            }
                        }
                        _ => {}
                    }
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    if retries >= max_retries {
                        return Err(anyhow::anyhow!("Transfer timed out"));
                    }
                    retries += 1;
                    log::warn!("Timeout, retrying... ({}/{})", retries, max_retries);

                    // Resend last ACK
                    let ack = Packet::Ack(block_num.wrapping_sub(1));
                    socket.send_to(&ack.serialize()?, server_addr)?;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }

    /// Upload a file to the server (WRQ - Write Request)
    pub fn put(&self, local_file: &Path, remote_file: &str) -> anyhow::Result<()> {
        log::info!("Uploading {} to {}", local_file.display(), remote_file);

        let mut file = File::open(local_file)?;
        let file_size = file.metadata()?.len();

        // Create local socket
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let mut server_addr = SocketAddr::new(self.server_ip, self.server_port);
        let mut tid_set = false;

        socket.set_read_timeout(Some(self.timeout))?;
        socket.set_write_timeout(Some(self.timeout))?;

        // Build options
        let options = self.build_options(file_size);

        // Send WRQ
        let wrq = Packet::Wrq {
            filename: remote_file.to_string(),
            mode: self.mode.clone(),
            options,
        };
        let bytes = wrq.serialize()?;
        socket.send_to(&bytes, server_addr)?;

        let mut block_num: u16 = 0;
        let mut retries = 0;
        let max_retries = 5;
        let mut finished = false;

        loop {
            let mut buf = vec![0; self.block_size as usize + 4];
            match socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    if !tid_set {
                        if src.ip() == self.server_ip {
                            server_addr = src;
                            tid_set = true;
                        } else {
                            continue;
                        }
                    } else if src != server_addr {
                        continue;
                    }

                    let packet = Packet::deserialize(&buf[..amt])?;
                    match packet {
                        Packet::Ack(block) => {
                            if block == block_num {
                                if finished {
                                    break;
                                }

                                block_num = block_num.wrapping_add(1);

                                // Read next block
                                let mut data = vec![0; self.block_size as usize];
                                let n = file.read(&mut data)?;
                                data.truncate(n);

                                if n < self.block_size as usize {
                                    finished = true;
                                }

                                // Send Data
                                let data_packet = Packet::Data { block_num, data };
                                socket.send_to(&data_packet.serialize()?, server_addr)?;

                                retries = 0;
                            }
                        }
                        Packet::Oack(_) => {
                            if block_num == 0 {
                                // OACK received, start sending data (block 1)
                                block_num = 1;

                                let mut data = vec![0; self.block_size as usize];
                                let n = file.read(&mut data)?;
                                data.truncate(n);

                                if n < self.block_size as usize {
                                    finished = true;
                                }

                                let data_packet = Packet::Data { block_num, data };
                                socket.send_to(&data_packet.serialize()?, server_addr)?;

                                retries = 0;
                            }
                        }
                        Packet::Error { code, msg } => {
                            return Err(anyhow::anyhow!("TFTP Error {:?}: {}", code, msg));
                        }
                        _ => {}
                    }
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    if retries >= max_retries {
                        return Err(anyhow::anyhow!("Transfer timed out"));
                    }
                    retries += 1;
                    log::warn!("Timeout, retrying... ({}/{})", retries, max_retries);

                    // Resend last packet (WRQ or Data)
                    if block_num == 0 {
                        // Resend WRQ
                        let wrq = Packet::Wrq {
                            filename: remote_file.to_string(),
                            mode: self.mode.clone(),
                            options: self.build_options(file_size),
                        };
                        socket.send_to(&wrq.serialize()?, server_addr)?;
                    } else {
                        // Resend Data
                        // We need to seek back in file?
                        // For simplicity in this refactor, we just error or warn.
                        // Proper retry for data requires caching the last data packet or seeking.
                        // Since we don't have the last data packet easily available here without restructuring,
                        // we will just log a warning that retry might fail if we don't resend data.
                        // Actually, we can seek back.

                        let offset = (block_num as u64 - 1) * (self.block_size as u64);
                        file.seek(std::io::SeekFrom::Start(offset))?;

                        let mut data = vec![0; self.block_size as usize];
                        let n = file.read(&mut data)?;
                        data.truncate(n);

                        let data_packet = Packet::Data { block_num, data };
                        socket.send_to(&data_packet.serialize()?, server_addr)?;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }
}
