use anyhow::Result;
use std::path::Path;

use super::super::fs::rm as fs_rm;
use super::super::types::PartitionTarget;
use super::super::utils::normalize_image_path;

pub fn rm(
    disk: &Path,
    target: &PartitionTarget,
    path: &str,
    recursive: bool,
    force: bool,
    _yes: bool,
) -> Result<()> {
    let image_path = normalize_image_path(path);
    let result = fs_rm(disk, target, &image_path, recursive);

    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            if force {
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}
