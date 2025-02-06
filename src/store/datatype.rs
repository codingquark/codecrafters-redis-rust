#[derive(Debug, Clone)]
pub enum DataType {
    String(String),
}

impl From<String> for DataType {
    fn from(s: String) -> Self {
        DataType::String(s)
    }
}

impl From<&str> for DataType {
    fn from(s: &str) -> Self {
        DataType::String(s.to_string())
    }
}

impl From<i64> for DataType {
    fn from(i: i64) -> Self {
        DataType::String(i.to_string())
    }
}

impl From<f64> for DataType {
    fn from(f: f64) -> Self {
        DataType::String(f.to_string())
    }
}

impl From<bool> for DataType {
    fn from(b: bool) -> Self {
        DataType::String(b.to_string())
    }
}

impl ToString for DataType {
    fn to_string(&self) -> String {
        match self {
            DataType::String(s) => s.clone(),
        }
    }
} 