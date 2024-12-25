use std::collections::HashMap;
use crate::error::Result;

pub struct Store {
    data: HashMap<String, String>,
}

impl Store {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self.data.get(key).cloned())
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        self.data.insert(key.to_string(), value.to_string());
        Ok(())
    }
}
