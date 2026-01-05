use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

pub fn run(port_name: &str, baud_rate: u32) -> anyhow::Result<()> {
    println!(
        "Connected to {} at {} baud. Press 'Ctrl + ]' to exit.",
        port_name, baud_rate
    );
    println!("---------------------------------------------------------------");

    // 1. Open Serial Port
    let mut serial_tx = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(10))
        .open()?;

    // Clone the port for the reading thread (serialport supports cloning)
    let mut serial_rx = serial_tx.try_clone()?;

    // 2. Enable Raw Mode
    enable_raw_mode()?;

    // Flag to coordinate shutdown
    let running = Arc::new(AtomicBool::new(true));
    let running_rx = running.clone();

    // 3. Spawn Thread: Serial -> Stdout
    // This thread reads bytes from the device and prints them to the terminal
    let rx_thread = thread::spawn(move || {
        let mut buffer = [0; 1024];
        let mut stdout = io::stdout();

        while running_rx.load(Ordering::Relaxed) {
            match serial_rx.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    // Handle line endings for display:
                    // Raw mode requires \r\n to move down and left.
                    // If the device sends just \n, we might need to fix it,
                    // but usually, we just pass through what we get.
                    // For a robust monitor, we often just write raw bytes.
                    let _ = stdout.write_all(&buffer[..n]);
                    let _ = stdout.flush();
                }
                Ok(_) => {} // Zero bytes read
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    // Timeout is normal, just loop again
                    continue;
                }
                Err(e) => {
                    // In raw mode, eprintln might not look right, but we try our best
                    // We can't easily print to stderr without messing up the terminal state
                    // so we just break.
                    let _ = write!(stdout, "\r\nError reading from serial: {}\r\n", e);
                    break;
                }
            }
        }
    });

    // 4. Main Loop: Stdin (Keyboard) -> Serial

    while running.load(Ordering::Relaxed) {
        // Poll for events to avoid blocking forever so we can check 'running'
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // Exit condition: Ctrl + ]
                    KeyCode::Char(']') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        running.store(false, Ordering::Relaxed);
                        break;
                    }

                    // Handle Enter key
                    KeyCode::Enter => {
                        // Most serial shells expect \r (Carriage Return)
                        serial_tx.write_all(b"\r")?;
                    }

                    // Pass through other characters
                    KeyCode::Char(c) => {
                        let mut buf = [0; 4];
                        let s = c.encode_utf8(&mut buf);
                        serial_tx.write_all(s.as_bytes())?;
                    }

                    // Handle Backspace (often tricky)
                    KeyCode::Backspace => {
                        // Send ASCII DEL (0x7F) or BS (0x08) depending on device
                        // Usually 0x08 (BS) or 0x7F (DEL). Let's try 0x08 first or 0x7F.
                        // Many terminals send 0x7F for backspace.
                        serial_tx.write_all(b"\x7F")?;
                    }

                    // You might need to handle arrows/special keys here if needed
                    _ => {}
                }
            }
        }
    }

    // 5. Cleanup
    disable_raw_mode()?;
    println!("\nDisconnected.");

    // Wait for RX thread to finish (optional, or just let it die with the process)
    // We set running to false, so it should exit on next timeout or read.
    let _ = rx_thread.join();

    Ok(())
}
