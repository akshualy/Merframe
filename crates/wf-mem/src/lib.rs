use std::ops::Range;

mod error;
mod name;
mod pe;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

pub use error::{MemError, Result};
pub use name::{basename, basename_matches};
pub use pe::image_size;

#[cfg(target_os = "linux")]
pub use linux::{LinuxProcess, find_process, parse_maps};
#[cfg(windows)]
pub use windows::{WindowsProcess, find_process};

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors the page protection flags"
)]
pub struct Region {
    pub range: Range<u64>,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub private: bool,
    pub path: Option<String>,
}

impl Region {
    pub fn size(&self) -> u64 {
        self.range.end - self.range.start
    }

    pub fn is_anonymous_heap(&self) -> bool {
        self.readable && self.writable && self.private && self.path.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
    pub base: u64,
    pub size: u64,
}

pub trait MemoryReader: Send + Sync {
    fn regions(&self) -> Result<Vec<Region>>;
    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<usize>;
    fn module(&self, name: &str) -> Result<Module>;

    fn read_exact(&self, addr: u64, buf: &mut [u8]) -> Result<()> {
        let n = self.read(addr, buf)?;
        if n == buf.len() {
            Ok(())
        } else {
            Err(MemError::Read {
                addr,
                len: buf.len(),
                source: std::io::Error::from(std::io::ErrorKind::UnexpectedEof),
            })
        }
    }

    fn read_u64(&self, addr: u64) -> Result<u64> {
        let mut b = [0u8; 8];
        self.read_exact(addr, &mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    fn read_vec(&self, addr: u64, len: usize) -> Result<Vec<u8>> {
        let mut v = vec![0u8; len];
        let n = self.read(addr, &mut v)?;
        v.truncate(n);
        Ok(v)
    }
}

pub const GAME_PROCESS: &str = "Warframe.x64.exe";

pub fn read_u32_le(bytes: &[u8], at: usize) -> Option<u32> {
    let raw = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(raw.try_into().ok()?))
}

pub fn read_u64_le(bytes: &[u8], at: usize) -> Option<u64> {
    let raw = bytes.get(at..at.checked_add(8)?)?;
    Some(u64::from_le_bytes(raw.try_into().ok()?))
}

#[cfg(target_os = "linux")]
pub fn open_game() -> Result<Box<dyn MemoryReader>> {
    let pid = find_process(GAME_PROCESS)?;
    Ok(Box::new(LinuxProcess::open(pid)?))
}

#[cfg(windows)]
pub fn open_game() -> Result<Box<dyn MemoryReader>> {
    let pid = find_process(GAME_PROCESS)?;
    Ok(Box::new(WindowsProcess::open(pid)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn little_endian_bounds() {
        let bytes = [1u8, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(read_u32_le(&bytes, 0), Some(1));
        assert_eq!(read_u32_le(&bytes, 4), Some(2));
        assert_eq!(read_u64_le(&bytes, 4), Some(2));
        assert_eq!(read_u32_le(&bytes, 9), None);
        assert_eq!(read_u64_le(&bytes, usize::MAX), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn access_denied_ptrace_hint() {
        assert_eq!(
            MemError::AccessDenied(7).to_string(),
            "Reading process 7 is not permitted, set kernel.yama.ptrace_scope to 0 or grant Merframe CAP_SYS_PTRACE"
        );
    }

    #[cfg(windows)]
    #[test]
    fn access_denied_admin_hint() {
        assert_eq!(
            MemError::AccessDenied(7).to_string(),
            "Reading process 7 is not permitted, run Merframe as administrator like the game"
        );
    }
}
