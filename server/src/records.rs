use serde::{Deserialize, Serialize};

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
