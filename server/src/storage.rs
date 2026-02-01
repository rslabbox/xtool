use log::info;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

pub const TEMP_DIR: &str = "temp";

#[derive(Clone, Serialize, Deserialize)]
pub enum StorageType {
    Qiniu(String), // key
    Memory(String), // content
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ContentType {
    Text,
    File,
}

impl ContentType {
#[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Text => "text/plain",
            ContentType::File => "application/zip",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: String,
    pub filename: Option<String>,
    pub content_type: ContentType,
    pub storage: StorageType,
    pub uploaded_at: u64,
}

pub fn init_temp_dir() -> io::Result<()> {
    let temp_path = PathBuf::from(TEMP_DIR);

    if temp_path.exists() {
        info!("Clearing temp directory...");
        for entry in fs::read_dir(&temp_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    } else {
        fs::create_dir(&temp_path)?;
    }
    info!("Temp directory ready");
    Ok(())
}
