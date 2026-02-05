use anyhow::{anyhow, Result};
use gpt::{disk::LogicalBlockSize, GptConfig};
use std::{fs::File, path::Path};

use super::types::{PartitionInfo, PartitionSpec, PartitionTarget};
use super::utils::{align_up, parse_u64_any};

const LB_SIZE_BYTES: u64 = 512;

pub fn open_gpt(disk: &Path, writable: bool) -> Result<gpt::GptDisk<File>> {
    GptConfig::new()
        .writable(writable)
        .logical_block_size(LogicalBlockSize::Lb512)
        .open(disk)
        .map_err(|e| anyhow!("failed to open GPT: {e}"))
}

pub fn map_partitions(gdisk: &gpt::GptDisk<File>) -> Result<Vec<PartitionInfo>> {
    let mut out = Vec::new();
    for (idx, part) in gdisk.partitions().iter() {
        if !part.is_used() {
            continue;
        }
        let start = part
            .bytes_start(LogicalBlockSize::Lb512)
            .map_err(|e| anyhow!("invalid partition start: {e}"))?;
        let size = part
            .bytes_len(LogicalBlockSize::Lb512)
            .map_err(|e| anyhow!("invalid partition size: {e}"))?;
        out.push(PartitionInfo {
            index: *idx,
            name: part.name.clone(),
            first_lba: part.first_lba,
            last_lba: part.last_lba,
            start_bytes: start,
            size_bytes: size,
        });
    }
    out.sort_by_key(|p| p.index);
    Ok(out)
}

pub fn parse_parameter_file(path: &Path) -> Result<Vec<PartitionSpec>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("failed to read parameter file {}: {e}", path.display()))?;

    let cmdline = content
        .lines()
        .find_map(|line| line.strip_prefix("CMDLINE:"))
        .map(|s| s.trim())
        .ok_or_else(|| anyhow!("CMDLINE not found in parameter file"))?;

    let mtd = cmdline
        .split_whitespace()
        .find_map(|part| part.strip_prefix("mtdparts="))
        .ok_or_else(|| anyhow!("mtdparts not found in CMDLINE"))?;

    let (_, parts_str) = mtd
        .split_once(':')
        .ok_or_else(|| anyhow!("invalid mtdparts format"))?;

    let mut specs = Vec::new();
    for raw in parts_str.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (size_off, name_part) = raw
            .split_once('(')
            .ok_or_else(|| anyhow!("invalid partition entry: {raw}"))?;
        let name_part = name_part.trim_end_matches(')');
        let (name, flags) = match name_part.split_once(':') {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (name_part.trim(), None),
        };

        let (size_str, off_str) = size_off
            .split_once('@')
            .ok_or_else(|| anyhow!("invalid partition entry: {raw}"))?;

        let grow = flags.is_some_and(|f| f.contains("grow"));
        let size_bytes = if size_str.trim() == "-" {
            None
        } else {
            Some(parse_u64_any(size_str.trim())?)
        };
        let offset_bytes = parse_u64_any(off_str.trim())?;

        specs.push(PartitionSpec {
            name: name.to_string(),
            offset_bytes,
            size_bytes,
            grow,
        });
    }

    Ok(specs)
}

pub fn resolve_partition_target(disk: &Path, part: Option<&str>) -> Result<PartitionTarget> {
    let disk_size = std::fs::metadata(disk)
        .map_err(|e| anyhow!("failed to stat disk {}: {e}", disk.display()))?
        .len();

    let Some(part) = part else {
        return Ok(PartitionTarget {
            offset_bytes: 0,
            size_bytes: disk_size,
        });
    };

    let gdisk = open_gpt(disk, false).map_err(|_| anyhow!("no GPT found on disk"))?;
    let parts = gdisk.partitions();

    let mut resolved: Option<(u32, gpt::partition::Partition)> = None;
    if let Ok(idx) = part.parse::<u32>() {
        if let Some(p) = parts.get(&idx) {
            resolved = Some((idx, p.clone()));
        }
    } else {
        for (idx, p) in parts.iter() {
            if p.is_used() && p.name == part {
                resolved = Some((*idx, p.clone()));
                break;
            }
        }
    }

    let (_index, part) = resolved.ok_or_else(|| {
        let list = parts
            .iter()
            .filter(|(_, p)| p.is_used())
            .map(|(idx, p)| format!("{}:{}", idx, p.name))
            .collect::<Vec<_>>()
            .join(", ");
        anyhow!("partition not found. available: {list}")
    })?;

    let start = part
        .bytes_start(LogicalBlockSize::Lb512)
        .map_err(|e| anyhow!("invalid partition start: {e}"))?;
    let size = part
        .bytes_len(LogicalBlockSize::Lb512)
        .map_err(|e| anyhow!("invalid partition size: {e}"))?;

    Ok(PartitionTarget {
        offset_bytes: start,
        size_bytes: size,
    })
}

pub fn align_partition_start(offset_bytes: u64, align_bytes: u64) -> u64 {
    let mut start = align_up(offset_bytes, align_bytes.max(LB_SIZE_BYTES));
    if !start.is_multiple_of(LB_SIZE_BYTES) {
        start = align_up(start, LB_SIZE_BYTES);
    }
    start
}

pub fn clamp_size_to_lba(size_bytes: u64) -> u64 {
    size_bytes - (size_bytes % LB_SIZE_BYTES)
}

pub fn lb_size_bytes() -> u64 {
    LB_SIZE_BYTES
}
