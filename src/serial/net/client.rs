use anyhow::{Result, Context};
// use log::info;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        println!(); // Ensure newline on exit
    }
}

pub async fn run(server: String, port: u16) -> Result<()> {
    let addr = format!("{}:{}", server, port);
    info!("Connecting to {}...", addr);
    
    let mut stream = TcpStream::connect(&addr).await.with_context(|| format!("Failed to connect to {}", addr))?;
    let (mut ri, mut wi) = stream.split();
    
    info!("Connected. Press 'Ctrl + ]' to exit.");
    
    // Enable raw mode
    enable_raw_mode()?;
    let _guard = RawModeGuard;

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Input thread (Blocking, for crossterm)
    std::thread::spawn(move || {
        loop {
             if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    // Ctrl + ] to exit
                    KeyCode::Char(']') | KeyCode::Char('5') 
                         if key.modifiers.contains(KeyModifiers::CONTROL) => {
                             break;
                    }
                    
                    KeyCode::Enter => {
                        let _ = tx.send(vec![b'\r']);
                    }
                    
                    KeyCode::Char(c) => {
                         let mut bytes = Vec::new();
                         if key.modifiers.contains(KeyModifiers::CONTROL) {
                             let byte = c as u8;
                             // Map a=1, z=26 for Ctrl+Key
                             if (b'a'..=b'z').contains(&byte) {
                                 bytes.push(byte - b'a' + 1);
                             } else if (b'A'..=b'Z').contains(&byte) {
                                 bytes.push(byte - b'A' + 1);
                             } else {
                                  // Basic fallback
                                  let mut b = [0; 4];
                                  bytes.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
                             }
                         } else {
                              let mut b = [0; 4];
                              bytes.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
                         }
                         let _ = tx.send(bytes);
                    }
                    
                    KeyCode::Backspace => {
                         let _ = tx.send(vec![0x08]);
                    }
                    
                    // Specific key mappings could be added here similar to a real terminal
                    _ => {}
                }
             }
        }
    });

    let mut buf = [0u8; 2048];
    let mut stdout = tokio::io::stdout();

    loop {
        tokio::select! {
            // Read from TCP and print to Stdout
            res = ri.read(&mut buf) => {
                match res {
                    Ok(n) if n > 0 => {
                        stdout.write_all(&buf[..n]).await?;
                        stdout.flush().await?;
                    }
                    Ok(_) => {
                        // EOF
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            
            // Read from Input Channel and write to TCP
            msg = rx.recv() => {
                match msg {
                    Some(data) => {
                        if wi.write_all(&data).await.is_err() {
                            break;
                        }
                        if wi.flush().await.is_err() { // Important for TCP immediateness
                            break;
                        }
                    }
                    None => {
                        // User requested exit
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
