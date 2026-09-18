use memchr::memmem;
use wf_mem::{MemoryReader, Region};

use crate::Result;
use crate::chunks::{Step, scan_regions, to_u64, to_usize};

pub const PATH_ANCHOR: &[u8] = b"\\EE.log\0";
pub const CURSOR_FIELD: usize = 8;
const SIZES: [usize; 5] = [0x1000, 0x2000, 0x4000, 0x8000, 0x1_0000];
const MIN_LINE_HEADS: usize = 3;
const MIN_REGION_SIZE: u64 = 0x1_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogBuffer {
    pub base: u64,
    pub size: usize,
}

pub fn is_log_text(byte: u8) -> bool {
    byte.is_ascii_graphic() || matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

pub fn is_line_head(data: &[u8]) -> bool {
    let mut at = 0;
    while data.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    if at == 0 || at > 7 || data.get(at) != Some(&b'.') {
        return false;
    }
    at += 1;
    let fraction = at;
    while data.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    if at - fraction != 3 || data.get(at) != Some(&b' ') {
        return false;
    }
    at += 1;
    let channel = at;
    while data.get(at).is_some_and(u8::is_ascii_alphabetic) {
        at += 1;
    }
    if at == channel || data.get(at) != Some(&b' ') || data.get(at + 1) != Some(&b'[') {
        return false;
    }
    at += 2;
    let level = at;
    while data.get(at).is_some_and(u8::is_ascii_alphabetic) {
        at += 1;
    }
    at != level
        && data.get(at) == Some(&b']')
        && data.get(at + 1) == Some(&b':')
        && data.get(at + 2) == Some(&b' ')
}

fn line_heads(pending: &[u8]) -> usize {
    memchr::memchr_iter(b'\n', pending)
        .filter(|at| is_line_head(&pending[at + 1..]))
        .count()
}

fn cursor_at(data: &[u8], field: usize) -> Option<usize> {
    let bytes = data.get(field..)?.first_chunk::<CURSOR_FIELD>()?;
    usize::try_from(u64::from_le_bytes(*bytes)).ok()
}

fn text_runs(data: &[u8]) -> impl Iterator<Item = (usize, usize)> {
    let mut at = 0;
    std::iter::from_fn(move || {
        while at < data.len() && !is_log_text(data[at]) {
            at += 1;
        }
        if at == data.len() {
            return None;
        }
        let start = at;
        while at < data.len() && is_log_text(data[at]) {
            at += 1;
        }
        Some((start, at))
    })
}

fn buffer_at(data: &[u8], base: usize, run_end: usize) -> Option<LogBuffer> {
    for size in SIZES {
        if base + size > run_end {
            return None;
        }
        if cursor_at(data, base + size).is_some_and(|c| c <= size)
            && line_heads(&data[base..base + size]) >= MIN_LINE_HEADS
        {
            return Some(LogBuffer {
                base: to_u64(base).ok()?,
                size,
            });
        }
    }
    None
}

pub fn locate(data: &[u8]) -> Option<LogBuffer> {
    text_runs(data).find_map(|(start, end)| buffer_at(data, start, end))
}

fn anchor_region<R: MemoryReader + ?Sized>(reader: &R) -> Result<Option<Region>> {
    let mut anchored = None;
    scan_regions(
        reader,
        MIN_REGION_SIZE,
        PATH_ANCHOR.len(),
        |region, _, data| {
            if memmem::find(data, PATH_ANCHOR).is_none() {
                return Ok(Step::Continue);
            }
            anchored = Some(region.clone());
            Ok(Step::Stop)
        },
    )?;
    Ok(anchored)
}

pub fn find_log_buffer<R: MemoryReader + ?Sized>(reader: &R) -> Result<Option<LogBuffer>> {
    let Some(region) = anchor_region(reader)? else {
        return Ok(None);
    };
    let data = reader.read_vec(region.range.start, to_usize(region.size())?)?;
    Ok(locate(&data).map(|found| LogBuffer {
        base: region.range.start + found.base,
        size: found.size,
    }))
}

pub fn read_pending<R: MemoryReader + ?Sized>(
    reader: &R,
    buffer: LogBuffer,
) -> Result<Option<Vec<u8>>> {
    let mut bytes = vec![0u8; buffer.size + CURSOR_FIELD];
    reader.read_exact(buffer.base, &mut bytes)?;
    let Some(cursor) = cursor_at(&bytes, buffer.size).filter(|c| *c <= buffer.size) else {
        return Ok(None);
    };
    bytes.truncate(cursor);
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;

    const SIZE: usize = 0x1000;

    fn log_text(lines: usize) -> Vec<u8> {
        let mut text = Vec::new();
        for index in 0..lines {
            let time = 1000.0 + f64::from(u32::try_from(index).unwrap()) * 0.25;
            text.extend_from_slice(
                format!("{time:.3} Sys [Info]: OnAgentCreated /Npc/Agent{index}\r\n").as_bytes(),
            );
        }
        text
    }

    fn buffer_region(pending: usize, size: usize, lead: usize) -> Vec<u8> {
        let mut data = vec![0u8; lead];
        let mut text = log_text(200);
        text.truncate(size);
        data.extend_from_slice(&text);
        data.resize(lead + size, b' ');
        data.extend_from_slice(&u64::try_from(pending).unwrap().to_le_bytes());
        data.extend_from_slice(&[0u8; 32]);
        data
    }

    #[test]
    fn text_run_and_cursor() {
        let data = buffer_region(0x800, SIZE, 0x40);
        let found = locate(&data).unwrap();
        assert_eq!(
            found,
            LogBuffer {
                base: 0x40,
                size: SIZE
            }
        );
    }

    #[test]
    fn empty_buffer() {
        let mut data = buffer_region(0x800, SIZE, 0x40);
        let field = 0x40 + SIZE;
        data[field..field + CURSOR_FIELD].copy_from_slice(&0u64.to_le_bytes());
        let found = locate(&data).unwrap();
        assert_eq!(found.base, 0x40);
        assert_eq!(found.size, SIZE);
    }

    #[test]
    fn cursor_past_buffer() {
        let mut data = buffer_region(0x800, SIZE, 0x40);
        let field = 0x40 + SIZE;
        data[field..field + CURSOR_FIELD]
            .copy_from_slice(&u64::try_from(SIZE + 1).unwrap().to_le_bytes());
        assert_eq!(locate(&data), None);
    }

    #[test]
    fn text_run_before_buffer() {
        let mut data = buffer_region(0x800, SIZE, 0x40);
        data[0x3f] = b'x';
        assert_eq!(locate(&data), None);
    }

    #[test]
    fn non_text_byte_breaks_run() {
        let mut data = buffer_region(0x800, SIZE, 0x40);
        data[0x40 + 0x400] = 0x01;
        assert_eq!(locate(&data), None);
    }

    #[test]
    fn too_few_line_heads() {
        let mut data = vec![0u8; 0x40];
        data.extend_from_slice(&[b'a'; SIZE]);
        data.extend_from_slice(&u64::try_from(0x800usize).unwrap().to_le_bytes());
        assert_eq!(locate(&data), None);
    }

    #[test]
    fn starts_mid_line() {
        let mut data = vec![0u8; 0x40];
        let mut text = b"cking 9 AllyLive 2 NeutralActive 0\r\n".to_vec();
        text.extend_from_slice(&log_text(200));
        text.truncate(SIZE);
        data.extend_from_slice(&text);
        data.extend_from_slice(&u64::try_from(0x600usize).unwrap().to_le_bytes());
        let found = locate(&data).unwrap();
        assert_eq!(found.base, 0x40);
        assert_eq!(found.size, SIZE);
    }

    #[test]
    fn region_with_log_path() {
        let mut region = Vec::new();
        region.extend_from_slice(b"C:\\users\\tenno\\AppData\\Local\\Warframe");
        region.extend_from_slice(PATH_ANCHOR);
        region.resize(0x200, 0);
        let base = u64::try_from(region.len()).unwrap();
        region.extend_from_slice(&buffer_region(0x800, SIZE, 0));
        region.resize(0x2_0000, 0);

        let reader = FakeReader::new(vec![(0x40_0000, region)]);
        let found = find_log_buffer(&reader).unwrap().unwrap();
        assert_eq!(found.base, 0x40_0000 + base);
        assert_eq!(found.size, SIZE);

        let pending = read_pending(&reader, found).unwrap().unwrap();
        assert_eq!(pending.len(), 0x800);
        assert!(pending.starts_with(b"1000.000 Sys [Info]: "));
    }

    #[test]
    fn region_without_log_path() {
        let mut region = buffer_region(0x800, SIZE, 0x40);
        region.resize(0x2_0000, 0);
        let reader = FakeReader::new(vec![(0x40_0000, region)]);
        assert_eq!(find_log_buffer(&reader).unwrap(), None);
    }

    #[test]
    fn cursor_out_of_range() {
        let mut region = vec![0u8; 0x40];
        region.extend_from_slice(&[b' '; SIZE]);
        region.extend_from_slice(&u64::MAX.to_le_bytes());
        let reader = FakeReader::new(vec![(0x1000, region)]);
        let buffer = LogBuffer {
            base: 0x1040,
            size: SIZE,
        };
        assert_eq!(read_pending(&reader, buffer).unwrap(), None);
    }

    #[test]
    fn line_heads() {
        assert!(is_line_head(b"1041.984 Script [Info]: WaveDefend.lua: x"));
        assert!(is_line_head(b"0.043 Sys [Warning]: x"));
        assert!(is_line_head(b"912.201 Snd [Error]: x"));
        assert!(!is_line_head(b"912.20 Sys [Info]: x"));
        assert!(!is_line_head(b"912.2011 Sys [Info]: x"));
        assert!(!is_line_head(b"912.201 Sys [Info] x"));
        assert!(!is_line_head(b"912.201 Sys []: x"));
        assert!(!is_line_head(b"912.201 [Info]: x"));
        assert!(!is_line_head(b".201 Sys [Info]: x"));
        assert!(!is_line_head(b""));
    }

    #[test]
    fn log_text_bytes() {
        assert!(is_log_text(b' '));
        assert!(is_log_text(b'~'));
        assert!(is_log_text(b'\r'));
        assert!(is_log_text(b'\n'));
        assert!(is_log_text(b'\t'));
        assert!(!is_log_text(0));
        assert!(!is_log_text(0x7f));
        assert!(!is_log_text(0xff));
    }
}
