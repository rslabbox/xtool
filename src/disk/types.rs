use serde::Serialize;

#[derive(Debug, Clone)]
pub struct PartitionTarget {
    pub offset_bytes: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct PartitionSpec {
    pub name: String,
    pub offset_bytes: u64,
    pub size_bytes: Option<u64>,
    pub grow: bool,
}

#[derive(Serialize)]
pub struct PartitionInfo {
    pub index: u32,
    pub name: String,
    pub first_lba: u64,
    pub last_lba: u64,
    pub start_bytes: u64,
    pub size_bytes: u64,
}

#[derive(Serialize)]
pub struct DiskInfo {
    pub disk: String,
    pub size_bytes: u64,
    pub partitions: Vec<PartitionInfo>,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    Host,
    Image,
}
