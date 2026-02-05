use anyhow::Result;
use std::path::Path;

use super::super::cli::FsType;
use super::super::fs::{mkfs_ext4, mkfs_fat32};
use super::super::types::PartitionTarget;
use super::super::utils::confirm_or_yes;

pub fn mkfs(
    disk: &Path,
    target: &PartitionTarget,
    fstype: FsType,
    label: Option<&str>,
    yes: bool,
) -> Result<()> {
    let prompt = format!("Format {}? This will erase data.", disk.display());
    confirm_or_yes(yes, &prompt)?;

    match fstype {
        FsType::Ext4 => mkfs_ext4(disk, target, label),
        FsType::Fat32 => mkfs_fat32(disk, target, label),
    }
}
