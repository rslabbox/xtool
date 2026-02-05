use anyhow::Result;
use std::path::Path;

use super::super::fs::mkdir as fs_mkdir;
use super::super::types::PartitionTarget;

pub fn mkdir(disk: &Path, target: &PartitionTarget, path: &str, parents: bool) -> Result<()> {
    fs_mkdir(disk, target, path, parents)
}
