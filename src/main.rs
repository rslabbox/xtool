mod config;
mod file;
mod serial;
mod tftp;

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::{error, info};
use std::path::PathBuf;

#[macro_use]
extern crate log;

#[derive(Parser)]
#[command(name = "xtool")]
#[command(version, about = "Amazing Tools", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a TFTP server
    Tftpd {
        /// IP address to listen on
        #[arg(short, long, default_value = "0.0.0.0")]
        ip: String,

        /// Port to listen on
        #[arg(short, long, default_value = "69")]
        port: u16,

        /// Root directory for TFTP files
        #[arg(value_name = "PATH")]
        path: PathBuf,

        /// Enable read-only mode
        #[arg(short, long)]
        read_only: bool,

        /// Use single port mode (useful for NAT environments)
        #[arg(short, long)]
        single_port: bool,
    },

    /// TFTP client - download or upload files
    Tftpc {
        #[command(subcommand)]
        action: tftp::client::TftpcAction,
    },

    /// File transfer - upload and download files
    File {
        #[command(subcommand)]
        action: file::FileAction,
    },

    /// Serial port tools - specify port to monitor, or use 'list' command
    Serial {
        /// Serial port name (e.g., COM1 or /dev/ttyUSB0). If not provided, will try to use config.
        #[arg(value_name = "UART")]
        uart: Option<String>,

        /// Baud rate
        #[arg(short, long)]
        baud: Option<u32>,

        #[command(subcommand)]
        subcommand: Option<serial::SerialSubcommand>,
    },

    /// Generate configuration file (.xtool.toml) in current directory
    Genconfig {
        /// Force overwrite existing configuration file
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    // Initialize logger, default info level, display file line number and time
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            let level_style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "[{} {level_style}{}{level_style:#} {}:{}] {level_style}{}{level_style:#}",
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                record.target(),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();

    let cli = Cli::parse();

    // Try to load configuration file
    let config_path = ".xtool.toml";
    let app_config = if std::path::Path::new(config_path).exists() {
        match config::AppConfig::load_from_file(config_path) {
            Ok(cfg) => {
                let abs_path = std::fs::canonicalize(config_path)
                    .unwrap_or_else(|_| std::path::PathBuf::from(config_path));
                info!("Using configuration file: {}", abs_path.display());
                Some(cfg)
            }
            Err(e) => {
                error!("Failed to load configuration file: {}, using defaults", e);
                None
            }
        }
    } else {
        None
    };

    match cli.command {
        Commands::Tftpd {
            ip,
            port,
            path,
            read_only,
            single_port,
        } => {
            tftp::server::run_with_config(
                ip,
                port,
                path,
                read_only,
                single_port,
                app_config.as_ref().and_then(|c| c.tftpd.clone()),
            )?;
        }

        Commands::Tftpc { action } => {
            // Client configuration merging is handled inside client::run_with_config
            tftp::client::run_with_config(
                action,
                app_config.as_ref().and_then(|c| c.tftpc.as_ref()),
            )?;
        }

        Commands::File { action } => {
            file::run(action)?;
        }

        Commands::Serial {
            uart,
            baud,
            subcommand,
        } => {
            serial::run(
                subcommand,
                uart,
                baud,
                app_config.as_ref().and_then(|c| c.serial.clone()),
            )?;
        }

        Commands::Genconfig { force } => {
            if let Err(e) = config::AppConfig::generate_config_file(force) {
                error!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
