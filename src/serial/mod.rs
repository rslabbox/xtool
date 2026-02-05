use anyhow::Result;
use clap::Subcommand;
use dialoguer::{theme::ColorfulTheme, Select};
use serialport::SerialPortType;

pub mod config;
pub mod list;
pub mod monitor;
pub mod net;

use config::SerialConfig;

#[derive(Subcommand)]
pub enum SerialSubcommand {
    /// List available serial ports
    List,
    /// Network setup server (Forward network to serial)
    Netd {
        /// Serial port name
        #[arg(value_name = "UART")]
        uart: Option<String>,
        /// Baud rate
        #[arg(short = 'b', long)]
        baud: Option<u32>,
        /// Listen port
        #[arg(short, long)]
        port: Option<u16>,
        /// Listen IP
        #[arg(short = 's', long)]
        bind: Option<String>,
    },
    /// Network connect client (Connect to serial server)
    Netc {
        /// Server IP
        #[arg(short, long)]
        server: String,
        /// Server Port
        #[arg(short, long, default_value = "5432")]
        port: u16,
    }
}

pub fn run(
    subcommand: Option<SerialSubcommand>,
    uart: Option<String>,
    baud: Option<u32>,
    config: Option<SerialConfig>,
) -> Result<()> {
    match subcommand {
        Some(SerialSubcommand::List) => return list::run(),
        Some(SerialSubcommand::Netd { uart, baud, port, bind }) => {
            let rt = tokio::runtime::Runtime::new()?;
            return rt.block_on(net::server::run(uart, baud, port, bind, config));
        },
        Some(SerialSubcommand::Netc { server, port }) => {
            let rt = tokio::runtime::Runtime::new()?;
            return rt.block_on(net::client::run(server, port));
        },
        _ => {}
    }

    // Default action: Monitor
    let final_uart = uart.or(config.as_ref().and_then(|c| c.uart.clone()));
    let final_baud = baud
        .or(config.as_ref().and_then(|c| c.baud))
        .unwrap_or(115200);

    let uart_name = match final_uart {
        Some(p) => p,
        None => {
            let ports = serialport::available_ports()?;
            if ports.is_empty() {
                anyhow::bail!("No serial ports found.");
            }

            let items: Vec<String> = ports
                .iter()
                .map(|p| {
                    let mut desc = p.port_name.clone();
                    if let SerialPortType::UsbPort(info) = &p.port_type
                        && let Some(product) = &info.product
                    {
                        desc.push_str(&format!(" - {}", product));
                    }
                    desc
                })
                .collect();

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select serial port")
                .default(0)
                .items(&items)
                .interact()?;

            ports[selection].port_name.clone()
        }
    };

    monitor::run(&uart_name, final_baud)
}
