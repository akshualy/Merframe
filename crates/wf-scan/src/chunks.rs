use std::ops::Range;

use wf_mem::{MemoryReader, Region};

use crate::{Result, ScanError};

pub const CHUNK: usize = 64 << 20;

pub enum Step {
    Continue,
    Stop,
}

fn read_chunks<R, F>(
    reader: &R,
    range: &Range<u64>,
    buffer: &mut [u8],
    overlap: usize,
    mut visit: F,
) -> Result<(u64, Step)>
where
    R: MemoryReader + ?Sized,
    F: FnMut(u64, &[u8]) -> Result<Step>,
{
    let mut bytes = 0u64;
    let mut pos = range.start;
    while pos < range.end {
        let want = to_usize((range.end - pos).min(to_u64(buffer.len())?))?;
        let read = match reader.read(pos, &mut buffer[..want]) {
            Ok(read) => read,
            Err(error) => {
                tracing::debug!(
                    addr = format!("{pos:#x}"),
                    %error,
                    "Region became unreadable mid-scan"
                );
                break;
            }
        };
        if read == 0 {
            break;
        }
        bytes += to_u64(read)?;
        if matches!(visit(pos, &buffer[..read])?, Step::Stop) {
            return Ok((bytes, Step::Stop));
        }
        if read <= overlap {
            break;
        }
        pos += to_u64(read - overlap)?;
    }
    Ok((bytes, Step::Continue))
}

pub fn scan_regions<R, F>(reader: &R, min_size: u64, overlap: usize, mut visit: F) -> Result<()>
where
    R: MemoryReader + ?Sized,
    F: FnMut(&Region, u64, &[u8]) -> Result<Step>,
{
    let mut buffer = vec![0u8; CHUNK];
    for region in reader.regions()? {
        if !region.is_anonymous_heap() || region.size() < min_size {
            continue;
        }
        let (_, step) = read_chunks(reader, &region.range, &mut buffer, overlap, |base, data| {
            visit(&region, base, data)
        })?;
        if matches!(step, Step::Stop) {
            return Ok(());
        }
    }
    Ok(())
}

pub fn to_usize(value: u64) -> Result<usize> {
    usize::try_from(value).map_err(|_| ScanError::Length(value))
}

pub fn to_u64(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| ScanError::Size(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;

    const BASE: u64 = 0x1000;

    #[test]
    fn overlapping_chunks() {
        let reader = FakeReader::new(vec![(BASE, (0..90u8).collect())]);
        let mut buffer = [0u8; 32];
        let mut seen = Vec::new();
        let (bytes, _) = read_chunks(&reader, &(BASE..BASE + 90), &mut buffer, 8, |base, data| {
            seen.push((base - BASE, data.len()));
            Ok(Step::Continue)
        })
        .unwrap();
        assert_eq!(seen, [(0, 32), (24, 32), (48, 32), (72, 18), (82, 8)]);
        assert_eq!(bytes, 122);
    }

    #[test]
    fn stop_ends_walk() {
        let reader = FakeReader::new(vec![
            (BASE, vec![1u8; 1 << 20]),
            (BASE + (2 << 20), vec![2u8; 1 << 20]),
        ]);
        let mut visits = 0;
        scan_regions(&reader, 0, 0, |_, _, data| {
            visits += 1;
            assert_eq!(data[0], 1);
            Ok(Step::Stop)
        })
        .unwrap();
        assert_eq!(visits, 1);
    }

    #[test]
    fn minimum_region_size() {
        let reader = FakeReader::new(vec![
            (BASE, vec![0u8; 1 << 20]),
            (BASE + (2 << 20), vec![0u8; 4 << 20]),
        ]);
        let mut bases = Vec::new();
        scan_regions(&reader, 2 << 20, 0, |_, base, _| {
            bases.push(base);
            Ok(Step::Continue)
        })
        .unwrap();
        assert_eq!(bases, [BASE + (2 << 20)]);
    }

    #[test]
    fn unreadable_region_ends_walk() {
        let reader = FakeReader::new(vec![(BASE, vec![0u8; 1 << 20])]);
        let mut visits = 0;
        let (bytes, _) = read_chunks(
            &reader,
            &(BASE + (1 << 20)..BASE + (2 << 20)),
            &mut [0u8; 64],
            0,
            |_, _| {
                visits += 1;
                Ok(Step::Continue)
            },
        )
        .unwrap();
        assert_eq!((visits, bytes), (0, 0));
    }
}
