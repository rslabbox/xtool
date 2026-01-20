use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::storage::FileRecord;

#[derive(Clone)]
pub struct AppState {
    pub files: Arc<Mutex<HashMap<String, FileRecord>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
