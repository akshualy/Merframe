#[derive(Debug, thiserror::Error)]
pub enum MemError {
    #[error("Process {0} not found")]
    ProcessNotFound(String),
    #[error("Module {0} not mapped")]
    ModuleNotFound(String),
    #[cfg_attr(
        not(windows),
        error(
            "Reading process {0} is not permitted, set kernel.yama.ptrace_scope to 0 or grant Merframe CAP_SYS_PTRACE"
        )
    )]
    #[cfg_attr(
        windows,
        error("Reading process {0} is not permitted, run Merframe as administrator like the game")
    )]
    AccessDenied(u32),
    #[error("Read at {addr:#x} ({len} bytes) failed: {source}")]
    Read {
        addr: u64,
        len: usize,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MemError>;
