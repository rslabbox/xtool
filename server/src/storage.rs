use log::info;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

pub const TEMP_DIR: &str = "temp";

#[derive(Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub remaining_downloads: u8,
    pub uploaded_at: u64,
    pub path: PathBuf,
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
