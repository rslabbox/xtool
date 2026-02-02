use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{records::FileRecord, qiniu::QiniuClient};

#[derive(Clone)]
pub struct AppState {
    pub files: Arc<Mutex<HashMap<String, FileRecord>>>,
    pub qiniu_config: Option<QiniuClient>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            qiniu_config: None,
        }
    }
}
