use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::Result;

pub struct Store {
    // TODO: Support multiple datatypes
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl Store {
    pub fn new() -> Self {
        Self { 
            data: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }

    pub async fn set(&self, key: &str, value: String) -> Result<()> {
        let mut data = self.data.write().await;
        data.insert(key.to_string(), value.to_string());
        Ok(())
    }
}
