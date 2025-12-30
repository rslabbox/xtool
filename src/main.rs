mod tftp;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "xtool")]
#[command(about = "Amazing Tools", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a TFTP server
    Tftpd {
        /// Port to listen on
        #[arg(short, long, default_value = "69")]
        port: u16,

        /// Root directory for TFTP files
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志，默认 info 等级，显示文件行数和时分秒
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "[{} {} {}:{}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                record.target(),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Tftpd { port, path } => {
            tftp::tftpd::run(port, path).await?;
        }
    }

    Ok(())
}
