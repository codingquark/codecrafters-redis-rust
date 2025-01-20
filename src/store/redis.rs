use std::collections::HashMap;
use std::time::{Instant, Duration};
use tokio::sync::RwLock;
use crate::error::Result;
use super::datatype::DataType;

pub struct Entry {
    value: DataType,
    expiry: Option<Instant>,
}

pub struct Store {
    data: RwLock<HashMap<String, Entry>>,
}

impl Store {
    pub fn new() -> Self {
        Self { 
            data: RwLock::new(HashMap::new())
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<DataType>> {
        let is_expired = {
            let data = self.data.read().await;
            if let Some(entry) = data.get(key) {
                if let Some(expiry) = entry.expiry {
                    Instant::now() > expiry
                } else {
                    false
                }
            } else {
                false
            }
        };

        if is_expired {
            let mut data = self.data.write().await;
            data.remove(key);
            return Ok(None);
        }

        let data = self.data.read().await;
        Ok(data.get(key).map(|entry| entry.value.clone()))
    }

    pub async fn set(&self, key: &str, value: DataType) -> Result<()> {
        let mut data = self.data.write().await;
        data.insert(key.to_string(), Entry { value, expiry: None });
        Ok(())
    }

    pub async fn set_ex(&self, key: &str, value: DataType, expiry: Duration) -> Result<()> {
        let mut data = self.data.write().await;
        let expiration = Instant::now() + expiry;
        data.insert(key.to_string(), Entry { value, expiry: Some(expiration) });
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }
}
