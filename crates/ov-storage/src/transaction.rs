//! Transaction manager for atomic operations

use ov_core::types::{TransactionRecord, TransactionStatus};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct TransactionManager {
    active: Mutex<HashMap<String, TransactionRecord>>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self { active: Mutex::new(HashMap::new()) }
    }

    pub fn begin(&self) -> TransactionRecord {
        let record = TransactionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            locks: Vec::new(),
            status: TransactionStatus::Init,
            init_info: HashMap::new(),
            rollback_info: HashMap::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        };
        self.active.lock().unwrap().insert(record.id.clone(), record.clone());
        record
    }
}
