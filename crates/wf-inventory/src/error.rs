#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    #[error("Inventory json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("$numberLong is not an integer: {0}")]
    NumberLong(String),
    #[error("Timestamp out of range: {0} ms")]
    Timestamp(i64),
}

pub type Result<T> = std::result::Result<T, InventoryError>;
