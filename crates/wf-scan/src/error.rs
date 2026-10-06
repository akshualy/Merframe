#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error(transparent)]
    Mem(#[from] wf_mem::MemError),
    #[error("Length {0:#x} exceeds the address space")]
    Length(u64),
}

pub type Result<T> = std::result::Result<T, ScanError>;
