#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("Log file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Watching the log directory failed: {0}")]
    Notify(#[from] notify::Error),
    #[error("Debug output channel: {0}")]
    DebugChannel(std::io::Error),
    #[error(transparent)]
    Mem(#[from] wf_mem::MemError),
    #[cfg(not(windows))]
    #[error(transparent)]
    Scan(#[from] wf_scan::ScanError),
}

pub type Result<T> = std::result::Result<T, LogError>;
