use anyhow::Result;
use clap::Subcommand;
use serde::Deserialize;
use std::path::PathBuf;

mod archive;
mod download;
mod upload;

const DEFAULT_SERVER_URL: &str = "http://localhost:3000";

#[derive(Subcommand)]
pub enum FileAction {
    /// Upload a file and return a token
    Send {
        /// File path to upload
        #[arg(value_name = "FILE", conflicts_with = "dirpath")]
        filepath: Option<PathBuf>,

        /// Directory path to upload (will be compressed)
        #[arg(short = 'd', long, value_name = "DIR", conflicts_with = "filepath")]
        dirpath: Option<PathBuf>,

        /// Download limit (1-10)
        #[arg(short, long, default_value_t = 1)]
        limit: u8,

        /// Server URL (e.g. http://localhost:3000)
        #[arg(short, long, default_value = DEFAULT_SERVER_URL)]
        server: String,
    },

    /// Download a file by token
    Get {
        /// File token
        #[arg(value_name = "TOKEN")]
        token: String,

        /// Output file path (defaults to server filename in current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Server URL (e.g. http://localhost:3000)
        #[arg(short, long, default_value = DEFAULT_SERVER_URL)]
        server: String,
    },
}

#[derive(Deserialize)]
struct UploadResponse {
    token: String,
    filename: String,
}

pub fn run(action: FileAction) -> Result<()> {
    match action {
        FileAction::Send {
            filepath,
            dirpath,
            limit,
            server,
        } => upload::send_file(&server, filepath.as_deref(), dirpath.as_deref(), limit),
        FileAction::Get {
            token,
            output,
            server,
        } => download::get_file(&server, &token, output.as_deref()),
    }
}
