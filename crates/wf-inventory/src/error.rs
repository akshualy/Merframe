#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    #[error("Inventory json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, InventoryError>;
