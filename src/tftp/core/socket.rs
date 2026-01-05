use super::Packet;
use std::{
    io::{Error as IoError, ErrorKind},
    net::{SocketAddr, UdpSocket},
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

const MAX_REQUEST_PACKET_SIZE: usize = 512;

/// Socket `trait` is used to allow building custom sockets to be used for
/// TFTP communication.
pub trait Socket: Send + Sync + 'static {
    /// Sends a [`Packet`] to the socket's connected remote [`Socket`].
    fn send(&self, packet: &Packet) -> anyhow::Result<()>;
    /// Sends a [`Packet`] to the specified remote [`Socket`].
    fn send_to(&self, packet: &Packet, to: &SocketAddr) -> anyhow::Result<()>;
    /// Receives a [`Packet`] from the socket's connected remote [`Socket`]. This
    /// function cannot handle large data packets due to the limited buffer size,
    /// so it is intended for only accepting incoming requests. For handling data
    /// packets, see [`Socket::recv_with_size()`].
    fn recv(&self) -> anyhow::Result<Packet> {
        self.recv_with_size(MAX_REQUEST_PACKET_SIZE)
    }
    /// Receives a data packet from the socket's connected remote, and returns the
    /// parsed [`Packet`]. The received packet can actually be of any type, however,
    /// this function also allows supplying the buffer size for an incoming request.
    fn recv_with_size(&self, size: usize) -> anyhow::Result<Packet>;
    /// Receives a [`Packet`] from any remote [`Socket`] and returns the [`SocketAddr`]
    /// of the remote [`Socket`]. This function cannot handle large data packets
    /// due to the limited buffer size, so it is intended for only accepting incoming
    /// requests. For handling data packets, see [`Socket::recv_from_with_size()`].
    fn recv_from(&self) -> anyhow::Result<(Packet, SocketAddr)> {
        self.recv_from_with_size(MAX_REQUEST_PACKET_SIZE)
    }
    /// Receives a data packet from any incoming remote request, and returns the
    /// parsed [`Packet`] and the requesting [`SocketAddr`]. The received packet can
    /// actually be of any type, however, this function also allows supplying the
    /// buffer size for an incoming request.
    fn recv_from_with_size(&self, size: usize) -> anyhow::Result<(Packet, SocketAddr)>;
    /// Returns the remote [`SocketAddr`] if it exists.
    fn remote_addr(&self) -> anyhow::Result<SocketAddr>;
    /// Sets the read timeout for the [`Socket`].
    fn set_read_timeout(&mut self, dur: Duration) -> anyhow::Result<()>;
    /// Sets the write timeout for the [`Socket`].
    fn set_write_timeout(&mut self, dur: Duration) -> anyhow::Result<()>;

    /// Sets the [`Socket`] as blocking or not.
    fn set_nonblocking(&mut self, nonblocking: bool) -> anyhow::Result<()>;
}

impl Socket for UdpSocket {
    fn send(&self, packet: &Packet) -> anyhow::Result<()> {
        self.send(&packet.serialize()?)?;

        Ok(())
    }

    fn send_to(&self, packet: &Packet, to: &SocketAddr) -> anyhow::Result<()> {
        self.send_to(&packet.serialize()?, to)?;

        Ok(())
    }

    fn recv_with_size(&self, size: usize) -> anyhow::Result<Packet> {
        let mut buf = vec![0; size + 4];
        let amt = self.recv(&mut buf)?;
        let packet = Packet::deserialize(&buf[..amt])?;

        Ok(packet)
    }

    fn recv_from_with_size(&self, size: usize) -> anyhow::Result<(Packet, SocketAddr)> {
        let mut buf = vec![0; size + 4];
        let (amt, addr) = self.recv_from(&mut buf)?;
        let packet = Packet::deserialize(&buf[..amt])?;

        Ok((packet, addr))
    }

    fn remote_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.peer_addr()?)
    }

    fn set_read_timeout(&mut self, dur: Duration) -> anyhow::Result<()> {
        UdpSocket::set_read_timeout(self, Some(dur))?;

        Ok(())
    }

    fn set_write_timeout(&mut self, dur: Duration) -> anyhow::Result<()> {
        UdpSocket::set_write_timeout(self, Some(dur))?;

        Ok(())
    }

    fn set_nonblocking(&mut self, nonblocking: bool) -> anyhow::Result<()> {
        UdpSocket::set_nonblocking(self, nonblocking)?;

        Ok(())
    }
}

/// ServerSocket `struct` is used as an abstraction layer for a server
/// [`Socket`]. This `struct` is used for abstraction of single socket
/// communication.
///
/// # Example
///
/// ```rust
/// use std::net::{SocketAddr, UdpSocket};
/// use std::str::FromStr;
/// use xtool::tftp::core::{Socket, ServerSocket, Packet};
/// use std::time::Duration;
///
/// let socket = ServerSocket::new(
///     UdpSocket::bind("127.0.0.1:0").unwrap(),
///     SocketAddr::from_str("127.0.0.1:50000").unwrap(),
///     Duration::from_secs(3)
/// );
/// socket.send(&Packet::Ack(1)).unwrap();
/// ```
pub struct ServerSocket {
    socket: UdpSocket,
    remote: SocketAddr,
    sender: Mutex<Sender<Packet>>,
    receiver: Mutex<Receiver<Packet>>,
    timeout: Duration,
    nonblocking: bool,
}

impl Socket for ServerSocket {
    fn send(&self, packet: &Packet) -> anyhow::Result<()> {
        self.send_to(packet, &self.remote)
    }

    fn send_to(&self, packet: &Packet, to: &SocketAddr) -> anyhow::Result<()> {
        self.socket.send_to(&packet.serialize()?, to)?;

        Ok(())
    }

    fn recv_with_size(&self, _size: usize) -> anyhow::Result<Packet> {
        if let Ok(receiver) = self.receiver.lock() {
            if self.nonblocking {
                if let Ok(packet) = receiver.try_recv() {
                    Ok(packet)
                } else {
                    Err(IoError::from(ErrorKind::WouldBlock).into())
                }
            } else if let Ok(packet) = receiver.recv_timeout(self.timeout) {
                Ok(packet)
            } else {
                Err(anyhow::anyhow!("Failed to receive"))
            }
        } else {
            Err(anyhow::anyhow!("Failed to lock mutex"))
        }
    }

    fn recv_from_with_size(&self, _size: usize) -> anyhow::Result<(Packet, SocketAddr)> {
        Ok((self.recv()?, self.remote))
    }

    fn remote_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.remote)
    }

    fn set_read_timeout(&mut self, dur: Duration) -> anyhow::Result<()> {
        self.timeout = dur;

        Ok(())
    }

    fn set_write_timeout(&mut self, dur: Duration) -> anyhow::Result<()> {
        self.socket.set_write_timeout(Some(dur))?;

        Ok(())
    }

    fn set_nonblocking(&mut self, nonblocking: bool) -> anyhow::Result<()> {
        self.nonblocking = nonblocking;
        self.socket.set_nonblocking(nonblocking)?;

        Ok(())
    }
}

impl ServerSocket {
    /// Creates a new [`ServerSocket`] from a [`UdpSocket`] and a remote [`SocketAddr`].
    pub fn new(socket: UdpSocket, remote: SocketAddr, timeout: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            socket,
            remote,
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
            timeout,
            nonblocking: false,
        }
    }

    /// Returns a [`Sender`] for sending [`Packet`]s to the remote [`Socket`].
    pub fn sender(&self) -> Sender<Packet> {
        self.sender.lock().unwrap().clone()
    }
}

impl<T: Socket + ?Sized> Socket for Box<T> {
    fn send(&self, packet: &Packet) -> anyhow::Result<()> {
        (**self).send(packet)
    }

    fn send_to(&self, packet: &Packet, to: &SocketAddr) -> anyhow::Result<()> {
        (**self).send_to(packet, to)
    }

    fn recv_with_size(&self, size: usize) -> anyhow::Result<Packet> {
        (**self).recv_with_size(size)
    }

    fn recv_from_with_size(&self, size: usize) -> anyhow::Result<(Packet, SocketAddr)> {
        (**self).recv_from_with_size(size)
    }

    fn remote_addr(&self) -> anyhow::Result<SocketAddr> {
        (**self).remote_addr()
    }

    fn set_read_timeout(&mut self, dur: Duration) -> anyhow::Result<()> {
        (**self).set_read_timeout(dur)
    }

    fn set_write_timeout(&mut self, dur: Duration) -> anyhow::Result<()> {
        (**self).set_write_timeout(dur)
    }

    fn set_nonblocking(&mut self, nonblocking: bool) -> anyhow::Result<()> {
        (**self).set_nonblocking(nonblocking)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    #[test]
    fn test_recv() {
        let socket = ServerSocket::new(
            UdpSocket::bind("127.0.0.1:0").unwrap(),
            SocketAddr::from_str("127.0.0.1:50000").unwrap(),
            Duration::from_secs(3),
        );

        socket.sender.lock().unwrap().send(Packet::Ack(1)).unwrap();

        let packet = socket.recv().unwrap();

        assert_eq!(packet, Packet::Ack(1));

        socket
            .sender
            .lock()
            .unwrap()
            .send(Packet::Data {
                block_num: 15,
                data: vec![0x01, 0x02, 0x03],
            })
            .unwrap();

        let packet = socket.recv().unwrap();

        assert_eq!(
            packet,
            Packet::Data {
                block_num: 15,
                data: vec![0x01, 0x02, 0x03]
            }
        );
    }
}
