use anyhow::Result;
use std::path::Path;

use super::super::fs::list_dir;
use super::super::types::PartitionTarget;

pub fn ls(disk: &Path, target: &PartitionTarget, path: &str) -> Result<()> {
    let entries = list_dir(disk, target, path)?;

    for entry in entries {
        if entry.is_dir {
            println!("{}/", entry.name);
        } else {
            println!("{}", entry.name);
        }
    }
    Ok(())
}
