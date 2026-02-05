use anyhow::Result;
use clap::Subcommand;
use serde::Deserialize;
use std::path::PathBuf;

mod archive;
mod download;
mod upload;

const DEFAULT_SERVER_URL: &str = "http://a.debin.cc:8080";

#[derive(Subcommand)]
pub enum FileAction {
    /// Upload a file and return a token
    Send {
        /// File or directory path to upload
        #[arg(value_name = "PATH", conflicts_with_all = ["message"])]
        path: Option<PathBuf>,

        /// Download limit (1-10)
        #[arg(short, long, default_value_t = 1)]
        limit: u8,

        /// Send a message as a message file (no file upload)
        #[arg(short = 'm', long, conflicts_with_all = ["path"])]
        message: Option<String>,

        /// Server URL (e.g. http://localhost:8080)
        #[arg(short, long, default_value = DEFAULT_SERVER_URL)]
        server: String,

        /// Encryption key for uploaded archives
        #[arg(short = 'k', long)]
        key: Option<String>,
    },

    /// Download a file by token
    Get {
        /// File token
        #[arg(value_name = "TOKEN")]
        token: String,

        /// Output file path (defaults to server filename in current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Server URL (e.g. http://localhost:8080)
        #[arg(short, long, default_value = DEFAULT_SERVER_URL)]
        server: String,

        /// Decryption key for encrypted archives
        #[arg(short = 'k', long)]
        key: Option<String>,
    },
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct UploadResponse {
    id: String,
    filename: Option<String>,
    upload_token: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
enum ContentType {
    Text,
    File,
}

#[derive(Deserialize, Debug)]
struct DownloadResponse {
    url: Option<String>,
    content: Option<String>,
    filename: Option<String>,
    content_type: ContentType,
}

pub fn run(action: FileAction) -> Result<()> {
    match action {
        FileAction::Send {
            path,
            limit,
            message,
            server,
            key,
        } => upload::send_file(
            &server,
            path.as_deref(),
            limit,
            message.as_deref(),
            key.as_deref(),
        ),
        FileAction::Get {
            token,
            output,
            server,
            key,
        } => download::get_file(&server, &token, output.as_deref(), key.as_deref()),
    }
}
