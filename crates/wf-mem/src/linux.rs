use std::fs::File;
use std::io::{Error, ErrorKind};
use std::os::unix::fs::FileExt;

use crate::name::basename_matches;
use crate::pe::image_size;
use crate::{MemError, MemoryReader, Module, Region, Result};

const EIO: i32 = 5;
const EFAULT: i32 = 14;
const HEADER_LEN: usize = 0x1000;

fn split_field(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    let (field, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    (!field.is_empty()).then_some((field, rest))
}

fn parse_line(line: &str) -> Option<Region> {
    let (bounds, rest) = split_field(line)?;
    let (perms, rest) = split_field(rest)?;
    let (_offset, rest) = split_field(rest)?;
    let (_device, rest) = split_field(rest)?;
    let (_inode, rest) = split_field(rest)?;

    let (start, end) = bounds.split_once('-')?;
    let start = u64::from_str_radix(start, 16).ok()?;
    let end = u64::from_str_radix(end, 16).ok()?;
    if end < start {
        return None;
    }

    let flags = perms.as_bytes();
    let path = rest.trim();

    Some(Region {
        range: start..end,
        readable: flags.first() == Some(&b'r'),
        writable: flags.get(1) == Some(&b'w'),
        executable: flags.get(2) == Some(&b'x'),
        private: flags.get(3) == Some(&b'p'),
        path: (!path.is_empty()).then(|| path.to_owned()),
    })
}

pub fn parse_maps(text: &str) -> Vec<Region> {
    text.lines().filter_map(parse_line).collect()
}

pub fn find_process(name: &str) -> Result<u32> {
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|d| d.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(cmdline) = std::fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let end = cmdline
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(cmdline.len());
        if basename_matches(&String::from_utf8_lossy(&cmdline[..end]), name) {
            return Ok(pid);
        }
    }
    Err(MemError::ProcessNotFound(name.to_owned()))
}

pub struct LinuxProcess {
    pid: u32,
    mem: File,
}

impl LinuxProcess {
    pub fn open(pid: u32) -> Result<Self> {
        let mem = File::open(format!("/proc/{pid}/mem")).map_err(|source| {
            if source.kind() == ErrorKind::PermissionDenied {
                MemError::AccessDenied(pid)
            } else {
                MemError::Io(source)
            }
        })?;
        Ok(Self { pid, mem })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl MemoryReader for LinuxProcess {
    fn regions(&self) -> Result<Vec<Region>> {
        let text = std::fs::read_to_string(format!("/proc/{}/maps", self.pid))?;
        Ok(parse_maps(&text))
    }

    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<usize> {
        match self.mem.read_at(buf, addr) {
            Ok(n) => Ok(n),
            Err(source) if matches!(source.raw_os_error(), Some(EIO | EFAULT)) => Ok(0),
            Err(source) => Err(MemError::Read {
                addr,
                len: buf.len(),
                source,
            }),
        }
    }

    fn module(&self, name: &str) -> Result<Module> {
        let base = self
            .regions()?
            .into_iter()
            .filter(|region| {
                region
                    .path
                    .as_deref()
                    .is_some_and(|path| basename_matches(path, name))
            })
            .map(|region| region.range.start)
            .min()
            .ok_or_else(|| MemError::ModuleNotFound(name.to_owned()))?;

        let header = self.read_vec(base, HEADER_LEN)?;
        let size = image_size(&header).ok_or_else(|| MemError::Read {
            addr: base,
            len: HEADER_LEN,
            source: Error::new(ErrorKind::InvalidData, "no PE header at module base"),
        })?;
        Ok(Module { base, size })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryReader, Region};

    const MAPS: &str = "\
6fffec2c0000-6fffec2c1000 r--p 00000000 103:01 52430780                  /media/test/content/SteamLibrary/steamapps/common/Warframe/Warframe.x64.exe
00020000-00420000 ---p 00000000 00:00 0
00440000-0045b000 rw-p 00000000 00:00 0
55557ea3e000-5555833e3000 rw-p 00000000 00:00 0                          [heap]
7ffceafd9000-7ffceaffe000 rw-p 00000000 00:00 0                          [stack]
00010000-00012000 r--s 00000000 103:03 10512541                          Z:\\wine\\nls\\l_intl.nls
";

    #[test]
    fn file_backed_region() {
        let regions = parse_maps(MAPS);
        let exe = &regions[0];
        assert_eq!(exe.range, 0x6fff_ec2c_0000..0x6fff_ec2c_1000);
        assert_eq!(exe.size(), 0x1000);
        assert!(exe.readable);
        assert!(!exe.writable);
        assert!(!exe.executable);
        assert!(exe.private);
        assert_eq!(
            exe.path.as_deref(),
            Some("/media/test/content/SteamLibrary/steamapps/common/Warframe/Warframe.x64.exe")
        );
        assert!(!exe.is_anonymous_heap());
    }

    #[test]
    fn guard_region() {
        let regions = parse_maps(MAPS);
        let guard = &regions[1];
        assert!(!guard.readable);
        assert!(!guard.writable);
        assert!(!guard.executable);
        assert!(guard.private);
        assert_eq!(guard.path, None);
        assert!(!guard.is_anonymous_heap());
    }

    #[test]
    fn anonymous_heap() {
        let regions = parse_maps(MAPS);
        let anon = &regions[2];
        assert_eq!(anon.range, 0x0044_0000..0x0045_b000);
        assert!(anon.readable);
        assert!(anon.writable);
        assert!(!anon.executable);
        assert!(anon.private);
        assert_eq!(anon.path, None);
        assert!(anon.is_anonymous_heap());
    }

    #[test]
    fn bracketed_names() {
        let regions = parse_maps(MAPS);
        assert_eq!(regions[3].path.as_deref(), Some("[heap]"));
        assert_eq!(regions[4].path.as_deref(), Some("[stack]"));
        assert!(!regions[3].is_anonymous_heap());
        assert!(!regions[4].is_anonymous_heap());
    }

    #[test]
    fn shared_mapping() {
        let regions = parse_maps(MAPS);
        let shared = &regions[5];
        assert!(shared.readable);
        assert!(!shared.private);
        assert_eq!(shared.path.as_deref(), Some("Z:\\wine\\nls\\l_intl.nls"));
    }

    #[test]
    fn region_count_and_malformed_lines() {
        assert_eq!(parse_maps(MAPS).len(), 6);
        assert!(parse_maps("").is_empty());
        assert!(parse_maps("garbage\nnot-a-map r--p\n").is_empty());
        assert!(parse_maps("6fffec2c0001-6fffec2c0000 r--p 0 00:00 0 ").is_empty());
    }

    #[test]
    fn reads_own_memory() {
        let pid = std::process::id();
        let target = LinuxProcess::open(pid).unwrap();
        assert_eq!(target.pid(), pid);

        let regions = target.regions().unwrap();
        assert!(regions.iter().any(|region| region.executable));
        assert!(regions.iter().any(Region::is_anonymous_heap));

        let probe = [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let addr = u64::try_from(probe.as_ptr().addr()).unwrap();
        assert_eq!(target.read_u64(addr).unwrap(), 0x8877_6655_4433_2211);
        assert_eq!(target.read_vec(addr, 4).unwrap(), probe[..4]);
    }

    #[test]
    fn foreign_process_denied() {
        if running_as_root() {
            return;
        }
        assert!(matches!(
            LinuxProcess::open(1),
            Err(crate::MemError::AccessDenied(1))
        ));
    }

    fn running_as_root() -> bool {
        std::fs::metadata("/proc/self")
            .is_ok_and(|meta| std::os::unix::fs::MetadataExt::uid(&meta) == 0)
    }

    #[test]
    fn unmapped_address() {
        let target = LinuxProcess::open(std::process::id()).unwrap();
        let mut buf = [0u8; 8];
        assert_eq!(target.read(0x1000, &mut buf).unwrap(), 0);
    }

    #[test]
    fn finds_test_binary() {
        let exe = std::env::current_exe().unwrap();
        let name = exe.file_name().and_then(std::ffi::OsStr::to_str).unwrap();
        assert_eq!(find_process(name).unwrap(), std::process::id());
        assert!(find_process("definitely-not-running-a1b2c3.exe").is_err());
    }
}
