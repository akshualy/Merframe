#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error(transparent)]
    Mem(#[from] wf_mem::MemError),
    #[error("Length {0:#x} exceeds the address space")]
    Length(u64),
    #[error("Chunk length {0} exceeds the address space")]
    Size(usize),
}

pub type Result<T> = std::result::Result<T, ScanError>;
