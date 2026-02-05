use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::super::fs::read_file;
use super::super::types::PartitionTarget;

pub fn cat(
    disk: &Path,
    target: &PartitionTarget,
    path: &str,
    bytes: Option<usize>,
    offset: Option<u64>,
) -> Result<()> {
    let offset = offset.unwrap_or(0);
    let data = read_file(disk, target, path, offset, bytes)?;

    let mut stdout = std::io::stdout();
    stdout.write_all(&data)?;
    Ok(())
}
