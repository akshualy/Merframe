use std::ffi::c_void;
use std::io::Error;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, MODULEENTRY32W, Module32FirstW, Module32NextW, PROCESSENTRY32W,
    Process32FirstW, Process32NextW, TH32CS_SNAPMODULE, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_IMAGE, MEM_PRIVATE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE, PAGE_EXECUTE_READ,
    PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS, PAGE_READONLY,
    PAGE_READWRITE, PAGE_WRITECOPY, VirtualQueryEx,
};
use windows_sys::Win32::System::ProcessStatus::GetMappedFileNameW;
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};

use crate::name::basename_matches;
use crate::{MemError, MemoryReader, Module, Region, Result};

const MAX_USER_ADDRESS: u64 = 0x0000_7fff_ffff_0000;
const PATH_LEN: usize = 512;
const PROTECT_MASK: u32 = 0xff;
const ERROR_INVALID_ADDRESS: i32 = 487;
const ERROR_PARTIAL_COPY: i32 = 299;
const ERROR_NOACCESS: i32 = 998;
const ERROR_ACCESS_DENIED: i32 = 5;

fn open_error(pid: u32, source: Error) -> MemError {
    if source.raw_os_error() == Some(ERROR_ACCESS_DENIED) {
        MemError::AccessDenied(pid)
    } else {
        MemError::Io(source)
    }
}

fn as_ptr(addr: u64) -> *const c_void {
    addr as *const c_void
}

fn wide_to_string(wide: &[u16]) -> String {
    let end = wide.iter().position(|u| *u == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..end])
}

fn protect_perms(protect: u32) -> (bool, bool, bool) {
    if protect & (PAGE_GUARD | PAGE_NOACCESS) != 0 {
        return (false, false, false);
    }
    let access = protect & PROTECT_MASK;
    let readable = matches!(
        access,
        PAGE_READONLY
            | PAGE_READWRITE
            | PAGE_WRITECOPY
            | PAGE_EXECUTE_READ
            | PAGE_EXECUTE_READWRITE
            | PAGE_EXECUTE_WRITECOPY
    );
    let writable = matches!(
        access,
        PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY
    );
    let executable = matches!(
        access,
        PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY
    );
    (readable, writable, executable)
}

struct Snapshot(HANDLE);

impl Snapshot {
    fn create(flags: u32, pid: u32) -> Result<Self> {
        // SAFETY: CreateToolhelp32Snapshot takes only by-value arguments and returns an owned handle.
        let handle = unsafe { CreateToolhelp32Snapshot(flags, pid) };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(MemError::Io(Error::last_os_error()));
        }
        Ok(Self(handle))
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        // SAFETY: self.0 is the handle from CreateToolhelp32Snapshot, closed here.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the size of a Win32 struct fits in u32"
)]
pub fn find_process(name: &str) -> Result<u32> {
    let snapshot = Snapshot::create(TH32CS_SNAPPROCESS, 0)?;
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    // SAFETY: the snapshot handle is live and entry is a writable PROCESSENTRY32W with dwSize set.
    let mut more = unsafe { Process32FirstW(snapshot.0, &raw mut entry) };
    while more != 0 {
        if basename_matches(&wide_to_string(&entry.szExeFile), name) {
            return Ok(entry.th32ProcessID);
        }
        // SAFETY: the snapshot handle is live and entry is a writable PROCESSENTRY32W.
        more = unsafe { Process32NextW(snapshot.0, &raw mut entry) };
    }
    Err(MemError::ProcessNotFound(name.to_owned()))
}

pub struct WindowsProcess {
    pid: u32,
    handle: HANDLE,
}

// SAFETY: a process handle from OpenProcess is a kernel object usable from any thread, and
// SAFETY: ReadProcessMemory, VirtualQueryEx and GetMappedFileNameW are thread-safe on a shared process handle.
unsafe impl Send for WindowsProcess {}
// SAFETY: same as the Send impl; every method takes &self and only reads through the handle.
unsafe impl Sync for WindowsProcess {}

impl WindowsProcess {
    pub fn open(pid: u32) -> Result<Self> {
        // SAFETY: OpenProcess takes only by-value arguments and returns an owned handle.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
        if handle.is_null() {
            return Err(open_error(pid, Error::last_os_error()));
        }
        Ok(Self { pid, handle })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    #[allow(clippy::cast_possible_truncation, reason = "PATH_LEN is 512")]
    fn mapped_file_name(&self, addr: u64) -> Option<String> {
        let mut buf = [0u16; PATH_LEN];
        // SAFETY: handle is live, buf is a writable array of PATH_LEN u16 and its length is passed.
        let len = unsafe {
            GetMappedFileNameW(self.handle, as_ptr(addr), buf.as_mut_ptr(), PATH_LEN as u32)
        };
        let len = usize::try_from(len).ok()?;
        if len == 0 || len > PATH_LEN {
            return None;
        }
        Some(wide_to_string(&buf[..len]))
    }
}

impl Drop for WindowsProcess {
    fn drop(&mut self) {
        // SAFETY: self.handle is the handle from OpenProcess, closed here.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

impl MemoryReader for WindowsProcess {
    fn regions(&self) -> Result<Vec<Region>> {
        let mut regions = Vec::new();
        let mut addr: u64 = 0;
        while addr < MAX_USER_ADDRESS {
            let mut info = MEMORY_BASIC_INFORMATION::default();
            // SAFETY: handle is live and info is a writable MEMORY_BASIC_INFORMATION of the passed size.
            let written = unsafe {
                VirtualQueryEx(
                    self.handle,
                    as_ptr(addr),
                    &raw mut info,
                    size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            if written == 0 {
                break;
            }

            let start = info.BaseAddress as u64;
            let end = start.saturating_add(info.RegionSize as u64);
            if info.State == MEM_COMMIT {
                let (readable, writable, executable) = protect_perms(info.Protect);
                regions.push(Region {
                    range: start..end,
                    readable,
                    writable,
                    executable,
                    private: info.Type == MEM_PRIVATE,
                    path: if info.Type == MEM_IMAGE {
                        self.mapped_file_name(start)
                    } else {
                        None
                    },
                });
            }
            if end <= addr {
                break;
            }
            addr = end;
        }
        Ok(regions)
    }

    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let mut done: usize = 0;
        // SAFETY: handle is live, buf is writable for buf.len() bytes and done receives the count.
        let ok = unsafe {
            ReadProcessMemory(
                self.handle,
                as_ptr(addr),
                buf.as_mut_ptr().cast::<c_void>(),
                buf.len(),
                &raw mut done,
            )
        };
        if ok != 0 {
            return Ok(done);
        }
        let source = Error::last_os_error();
        match source.raw_os_error() {
            Some(ERROR_PARTIAL_COPY | ERROR_NOACCESS | ERROR_INVALID_ADDRESS) => Ok(done),
            _ => Err(MemError::Read {
                addr,
                len: buf.len(),
                source,
            }),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the size of a Win32 struct fits in u32"
    )]
    fn module(&self, name: &str) -> Result<Module> {
        let snapshot = Snapshot::create(TH32CS_SNAPMODULE, self.pid)?;
        let mut entry = MODULEENTRY32W {
            dwSize: size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };

        // SAFETY: the snapshot handle is live and entry is a writable MODULEENTRY32W with dwSize set.
        let mut more = unsafe { Module32FirstW(snapshot.0, &raw mut entry) };
        while more != 0 {
            if basename_matches(&wide_to_string(&entry.szModule), name) {
                return Ok(Module {
                    base: entry.modBaseAddr as u64,
                    size: u64::from(entry.modBaseSize),
                });
            }
            // SAFETY: the snapshot handle is live and entry is a writable MODULEENTRY32W.
            more = unsafe { Module32NextW(snapshot.0, &raw mut entry) };
        }
        Err(MemError::ModuleNotFound(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemError;
    use windows_sys::Win32::System::Memory::{
        PAGE_EXECUTE_READ, PAGE_GUARD, PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE,
    };

    #[test]
    fn open_error_kinds() {
        assert!(matches!(
            open_error(42, std::io::Error::from_raw_os_error(5)),
            MemError::AccessDenied(42)
        ));
        assert!(matches!(
            open_error(42, std::io::Error::from_raw_os_error(87)),
            MemError::Io(_)
        ));
    }

    #[test]
    fn wide_string_nul() {
        let wide = [0x0057u16, 0x0046, 0x0000, 0x0058];
        assert_eq!(wide_to_string(&wide), "WF");
        assert_eq!(wide_to_string(&[0x0041u16]), "A");
        assert_eq!(wide_to_string(&[]), "");
    }

    #[test]
    fn protect_perms_table() {
        assert_eq!(protect_perms(PAGE_READONLY), (true, false, false));
        assert_eq!(protect_perms(PAGE_READWRITE), (true, true, false));
        assert_eq!(protect_perms(PAGE_EXECUTE_READ), (true, false, true));
        assert_eq!(protect_perms(PAGE_NOACCESS), (false, false, false));
        assert_eq!(
            protect_perms(PAGE_READWRITE | PAGE_GUARD),
            (false, false, false)
        );
    }
}
