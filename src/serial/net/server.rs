use anyhow::{Result, Context};
use crate::serial::config::SerialConfig;
use log::{info, error};
use tokio::net::TcpListener;
use tokio_serial::SerialPortBuilderExt;

pub async fn run(uart: Option<String>, baud: Option<u32>, port: Option<u16>, bind: Option<String>, config: Option<SerialConfig>) -> Result<()> {
    // Resolve UART and Baud
    let final_uart = uart.or(config.as_ref().and_then(|c| c.uart.clone()));
    let final_baud = baud.or(config.as_ref().and_then(|c| c.baud)).unwrap_or(115200);

    // Resolve Port and Bind IP
    let final_port = port.or(config.as_ref().and_then(|c| c.net_port)).unwrap_or(5432);
    let final_bind = bind.or(config.as_ref().and_then(|c| c.net_bind.clone())).unwrap_or_else(|| "0.0.0.0".to_string());

    let uart_name = final_uart.ok_or_else(|| anyhow::anyhow!("Serial port not specified. Please use UART argument or config file."))?;

    info!("Starting Netd: Serial <-> TCP Server");
    info!("Serial Port: {}, Baud: {}", uart_name, final_baud);

    // Open Serial Port
    let mut serial_stream = tokio_serial::new(&uart_name, final_baud)
        .open_native_async()
        .with_context(|| format!("Failed to open serial port {}", uart_name))?;

    #[cfg(unix)]
    {
        // use tokio_serial::SerialPort;
        // serial_stream.set_exclusive(false).ok(); 
        // NOTE: tokio-serial 5.4's SerialStream implements SerialPort trait. 
        // If the compiler says unused import, it might be due to trait scope rules or how tokio-serial re-exports it.
        // To be safe and avoid warning, we can access it via the trait path if needed, or remove if truly unused.
        // However, usually we need the trait in scope to call set_exclusive.
        // If it compiles without use, let's try removing it.
        // But `set_exclusive` is part of `serialport::SerialPort` trait.
        // tokio_serial re-exports it. 
        serial_stream.set_exclusive(false).ok(); 
    }

    let addr = format!("{}:{}", final_bind, final_port);
    let listener = TcpListener::bind(&addr).await.with_context(|| format!("Failed to bind to {}", addr))?;
    
    info!("Listening on {}", addr);
    info!("Ready to accept connections...");

    loop {
        match listener.accept().await {
            Ok((mut socket, peer_addr)) => {
                info!("Client connected from {}", peer_addr);
                
                // Bridge the two streams
                // We borrow serial_stream mutably. Ideally we want to lock it?
                // But since we are in a loop and await, we implicitly lock it for this connection.
                // Other connections will wait in accept backlog (or we won't call accept until this finishes).
                
                match tokio::io::copy_bidirectional(&mut socket, &mut serial_stream).await {
                    Ok((to_client, to_serial)) => {
                        info!("Session ended. Sent to client: {} bytes, Sent to serial: {} bytes", to_client, to_serial);
                    },
                    Err(e) => {
                        error!("Session error with {}: {}", peer_addr, e);
                    }
                }
                
                info!("Waiting for next connection...");
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
