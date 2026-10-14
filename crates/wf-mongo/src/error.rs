#[derive(Debug, thiserror::Error)]
pub enum MongoError {
    #[error("$numberLong is not an integer: {0}")]
    NumberLong(String),
    #[error("Timestamp out of range: {0} ms")]
    Timestamp(i64),
}

pub type Result<T> = std::result::Result<T, MongoError>;
