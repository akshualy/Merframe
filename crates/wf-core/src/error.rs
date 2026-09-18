use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Json: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Inventory(#[from] wf_inventory::InventoryError),
    #[error(transparent)]
    GameData(#[from] wf_data::DataError),
    #[error(transparent)]
    WorldState(#[from] wf_worldstate::WorldStateError),
    #[error("Compressing the inventory failed: {0}")]
    Compress(#[source] std::io::Error),
    #[error("Decompressing {path} failed: {source}")]
    Decompress {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Reading {path} failed: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Writing {path} failed: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Store schema version {0} is newer than this build understands")]
    SchemaVersion(i64),
    #[error("Stored timestamp {0} ms is out of range")]
    Timestamp(i64),
}
