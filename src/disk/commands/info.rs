use anyhow::Result;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::super::gpt::{map_partitions, open_gpt};
use super::super::types::DiskInfo;

pub fn info(disk: &Path, json: bool) -> Result<()> {
    let disk_size = std::fs::metadata(disk)?.len();

    let partitions = match open_gpt(disk, false) {
        Ok(gdisk) => map_partitions(&gdisk)?,
        Err(_) => Vec::new(),
    };

    if json {
        let info = DiskInfo {
            disk: disk.display().to_string(),
            size_bytes: disk_size,
            partitions,
        };
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    println!(
        "Disk: {} ({} M, {} bytes)",
        disk.display(),
        format_mib(disk_size),
        disk_size
    );
    if partitions.is_empty() {
        println!("No GPT partitions found.");
        let fs_type = detect_fs_type(disk)?;
        println!(
            "Filesystem: {}",
            fs_type.as_deref().unwrap_or("unknown")
        );
        return Ok(());
    }

    for p in partitions {
        println!(
            "{:>3} {:<16} start={} M size={} M",
            p.index,
            p.name,
            format_mib(p.start_bytes),
            format_mib(p.size_bytes)
        );
    }
    Ok(())
}

fn format_mib(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if bytes.is_multiple_of(MIB) {
        format!("{}", bytes / MIB)
    } else {
        format!("{:.1}", bytes as f64 / MIB as f64)
    }
}

fn detect_fs_type(disk: &Path) -> Result<Option<String>> {
    let mut file = OpenOptions::new().read(true).open(disk)?;

    let mut ext_magic = [0u8; 2];
    if file.seek(SeekFrom::Start(1024 + 56)).is_ok()
        && file.read_exact(&mut ext_magic).is_ok()
        && u16::from_le_bytes(ext_magic) == 0xEF53
    {
        return Ok(Some("ext4".to_string()));
    }

    let mut boot = [0u8; 512];
    let read = file.read(&mut boot)?;
    if read >= 512 && boot[510] == 0x55 && boot[511] == 0xAA {
        if boot.get(82..87) == Some(b"FAT32") {
            return Ok(Some("fat32".to_string()));
        }
        if boot.get(54..59) == Some(b"FAT16") {
            return Ok(Some("fat16".to_string()));
        }
        if boot.get(54..59) == Some(b"FAT12") {
            return Ok(Some("fat12".to_string()));
        }
    }

    Ok(None)
}
