use anyhow::Result;
use clap::Subcommand;
use dialoguer::{theme::ColorfulTheme, Select};
use serialport::SerialPortType;

pub mod config;
pub mod list;
pub mod monitor;

use config::SerialConfig;

#[derive(Subcommand)]
pub enum SerialSubcommand {
    /// List available serial ports
    List,
}

pub fn run(
    subcommand: Option<SerialSubcommand>,
    port: Option<String>,
    baud: Option<u32>,
    config: Option<SerialConfig>,
) -> Result<()> {
    if let Some(SerialSubcommand::List) = subcommand {
        return list::run();
    }

    // Default action: Monitor
    let final_port = port.or(config.as_ref().and_then(|c| c.port.clone()));
    let final_baud = baud
        .or(config.as_ref().and_then(|c| c.baud))
        .unwrap_or(115200);

    let port_name = match final_port {
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
                    if let SerialPortType::UsbPort(info) = &p.port_type {
                        if let Some(product) = &info.product {
                            desc.push_str(&format!(" - {}", product));
                        }
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

    monitor::run(&port_name, final_baud)
}
