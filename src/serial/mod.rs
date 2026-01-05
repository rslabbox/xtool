use anyhow::Result;
use clap::Subcommand;

pub mod config;
pub mod list;
pub mod monitor;

use config::SerialConfig;

#[derive(Subcommand)]
pub enum SerialAction {
    /// List available serial ports
    List,
    /// Open a serial port for monitoring/interaction
    Monitor {
        /// Serial port name (e.g., COM1 or /dev/ttyUSB0)
        #[arg(value_name = "PORT")]
        port: Option<String>,
        /// Baud rate
        #[arg(short, long)]
        baud: Option<u32>,
    },
}

pub fn run(action: SerialAction, config: Option<SerialConfig>) -> Result<()> {
    match action {
        SerialAction::List => list::run(),
        SerialAction::Monitor { port, baud } => {
            let final_port = port.or(config.as_ref().and_then(|c| c.port.clone()));
            let final_baud = baud
                .or(config.as_ref().and_then(|c| c.baud))
                .unwrap_or(115200);

            let port_name = final_port.ok_or_else(|| {
                anyhow::anyhow!("Serial port not specified. Use argument or set in config file.")
            })?;

            monitor::run(&port_name, final_baud)
        }
    }
}
