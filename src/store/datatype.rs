#[derive(Debug, Clone)]
pub enum DataType {
    String(String),
    Integer(i64),
    Double(f64),
    Boolean(bool),
    Null,
} 