use std::io;

use wf_mem::{MemError, MemoryReader, Module, Region};

pub struct FakeReader {
    regions: Vec<(u64, Vec<u8>)>,
    image: Option<u64>,
}

impl FakeReader {
    pub fn new(regions: Vec<(u64, Vec<u8>)>) -> Self {
        Self {
            regions,
            image: None,
        }
    }

    pub fn with_image(image: (u64, Vec<u8>), mut regions: Vec<(u64, Vec<u8>)>) -> Self {
        let start = image.0;
        regions.push(image);
        Self {
            regions,
            image: Some(start),
        }
    }

    fn end(start: u64, data: &[u8]) -> u64 {
        start + u64::try_from(data.len()).expect("region length")
    }
}

impl MemoryReader for FakeReader {
    fn regions(&self) -> wf_mem::Result<Vec<Region>> {
        Ok(self
            .regions
            .iter()
            .map(|(start, data)| Region {
                range: *start..Self::end(*start, data),
                readable: true,
                writable: true,
                executable: false,
                private: true,
                path: None,
            })
            .collect())
    }

    fn read(&self, addr: u64, buf: &mut [u8]) -> wf_mem::Result<usize> {
        for (start, data) in &self.regions {
            if addr < *start || addr >= Self::end(*start, data) {
                continue;
            }
            let offset = usize::try_from(addr - *start).expect("region offset");
            let len = buf.len().min(data.len() - offset);
            buf[..len].copy_from_slice(&data[offset..offset + len]);
            return Ok(len);
        }
        Err(MemError::Read {
            addr,
            len: buf.len(),
            source: io::Error::from(io::ErrorKind::NotFound),
        })
    }

    fn module(&self, name: &str) -> wf_mem::Result<Module> {
        self.regions
            .iter()
            .find(|(start, _)| self.image == Some(*start))
            .map(|(start, data)| Module {
                base: *start,
                size: Self::end(*start, data) - start,
            })
            .ok_or_else(|| MemError::ModuleNotFound(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wf_mem::MemoryReader;

    #[test]
    fn reads_inside_region_only() {
        let reader = FakeReader::new(vec![(0x1000, (0..64u8).collect())]);
        let regions = reader.regions().unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].size(), 64);
        assert!(regions[0].is_anonymous_heap());
        let mut buf = [0u8; 8];
        assert_eq!(reader.read(0x1004, &mut buf).unwrap(), 8);
        assert_eq!(buf, [4, 5, 6, 7, 8, 9, 10, 11]);
        let mut tail = [0u8; 8];
        assert_eq!(reader.read(0x1000 + 60, &mut tail).unwrap(), 4);
        assert!(reader.read(0x9000, &mut buf).is_err());
    }
}
