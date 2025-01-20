use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::error::Result;
use super::datatype::DataType;

pub struct Store {
    data: RwLock<HashMap<String, DataType>>,
}

impl Store {
    pub fn new() -> Self {
        Self { 
            data: RwLock::new(HashMap::new())
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<DataType>> {
        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }

    pub async fn set(&self, key: &str, value: DataType) -> Result<()> {
        let mut data = self.data.write().await;
        data.insert(key.to_string(), value);
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }
}
