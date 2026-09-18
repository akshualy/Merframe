use crate::read_u32_le;

const DOS_MAGIC: &[u8] = b"MZ";
const PE_MAGIC: &[u8] = b"PE\0\0";
const E_LFANEW: usize = 0x3c;
const SIZE_OF_IMAGE: usize = 0x50;

pub fn image_size(header: &[u8]) -> Option<u64> {
    if header.get(..DOS_MAGIC.len())? != DOS_MAGIC {
        return None;
    }
    let pe = usize::try_from(read_u32_le(header, E_LFANEW)?).ok()?;
    let pe_end = pe.checked_add(PE_MAGIC.len())?;
    if header.get(pe..pe_end)? != PE_MAGIC {
        return None;
    }
    let size = read_u32_le(header, pe.checked_add(SIZE_OF_IMAGE)?)?;
    (size != 0).then(|| u64::from(size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Vec<u8> {
        let mut h = vec![0u8; 0x100];
        h[0..2].copy_from_slice(b"MZ");
        h[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        h[0x40..0x44].copy_from_slice(b"PE\0\0");
        h[0x90..0x94].copy_from_slice(&0x0730_0000u32.to_le_bytes());
        h
    }

    #[test]
    fn size_of_image() {
        assert_eq!(image_size(&header()), Some(0x0730_0000));
    }

    #[test]
    fn missing_dos_magic() {
        let mut h = header();
        h[0] = b'X';
        assert_eq!(image_size(&h), None);
    }

    #[test]
    fn missing_pe_magic() {
        let mut h = header();
        h[0x40] = b'Q';
        assert_eq!(image_size(&h), None);
    }

    #[test]
    fn zero_size_or_truncated() {
        let mut h = header();
        h[0x90..0x94].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(image_size(&h), None);
        assert_eq!(image_size(&header()[..0x80]), None);
        assert_eq!(image_size(&[]), None);
    }

    #[test]
    fn e_lfanew_out_of_range() {
        let mut h = header();
        h[0x3c..0x40].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(image_size(&h), None);
    }
}
