use wf_mem::{MemoryReader, read_u32_le, read_u64_le};

const USERDATA_TAG: u8 = 9;
const RECORD_LEN: u32 = 0x58;
const RECORD_HEAD: usize = 0x40;
const FINGERPRINT: usize = 0x30;
const TEXT_FIELD: usize = 16;
const INLINE_TEXT: usize = 15;
const LONG_TEXT: u8 = 0xff;
const LONG_LEN_MASK: u32 = 0x00ff_ffff;
const MAX_FINGERPRINT: usize = 4096;

pub fn upgrade_fingerprint<R: MemoryReader + ?Sized>(reader: &R, record: u64) -> Option<String> {
    let mut head = [0u8; RECORD_HEAD];
    reader.read_exact(record, &mut head).ok()?;
    if head[0] != USERDATA_TAG || read_u32_le(&head, 4)? != RECORD_LEN {
        return None;
    }

    let field = &head[FINGERPRINT..FINGERPRINT + TEXT_FIELD];
    let text = match field[INLINE_TEXT] {
        LONG_TEXT => {
            let len = usize::try_from(read_u32_le(field, 8)? & LONG_LEN_MASK).ok()?;
            if len > MAX_FINGERPRINT {
                return None;
            }

            reader.read_vec(read_u64_le(field, 0)?, len).ok()?
        }
        spare => field[..INLINE_TEXT.checked_sub(usize::from(spare))?].to_vec(),
    };
    String::from_utf8(text).ok().filter(|text| !text.is_empty())
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub(crate) fn record(fingerprint: &str, text_at: Option<u64>) -> Vec<u8> {
        let mut record = vec![0u8; usize::try_from(RECORD_LEN).unwrap() + 0x10];
        record[0] = USERDATA_TAG;
        record[4..8].copy_from_slice(&RECORD_LEN.to_le_bytes());
        let field = &mut record[FINGERPRINT..FINGERPRINT + TEXT_FIELD];
        if let Some(address) = text_at {
            field[..8].copy_from_slice(&address.to_le_bytes());
            let len = u32::try_from(fingerprint.len()).unwrap() | 0xf000_0000;
            field[8..12].copy_from_slice(&len.to_le_bytes());
            field[INLINE_TEXT] = LONG_TEXT;
        } else {
            field[..fingerprint.len()].copy_from_slice(fingerprint.as_bytes());
            field[INLINE_TEXT] = u8::try_from(INLINE_TEXT - fingerprint.len()).unwrap();
        }
        record
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::record;
    use super::*;
    use crate::fake::FakeReader;

    const RECORD: u64 = 0x6000;
    const TEXT: u64 = 0x7000;
    const ROLL: &str = "{\"compat\":\"/Lotus/Weapons/Tenno/Shotgun/DoubleBarrelShotgun\",\"lim\":682347173,\"lvlReq\":11,\"rerolls\":60,\"pol\":\"AP_DEFENSE\",\"buffs\":[{\"Tag\":\"WeaponToxinDamageMod\",\"Value\":979842983}],\"curses\":[]}";

    #[test]
    fn inline_fingerprint() {
        let reader = FakeReader::new(vec![(RECORD, record("{\"lvl\":5}", None))]);
        assert_eq!(
            upgrade_fingerprint(&reader, RECORD).as_deref(),
            Some("{\"lvl\":5}")
        );
    }

    #[test]
    fn long_fingerprint() {
        let reader = FakeReader::new(vec![
            (RECORD, record(ROLL, Some(TEXT))),
            (TEXT, ROLL.as_bytes().to_vec()),
        ]);
        assert_eq!(upgrade_fingerprint(&reader, RECORD).as_deref(), Some(ROLL));
    }

    #[test]
    fn empty_and_foreign_records() {
        let reader = FakeReader::new(vec![(RECORD, record("", None))]);
        assert_eq!(upgrade_fingerprint(&reader, RECORD), None);
        let mut other = record("{\"lvl\":5}", None);
        other[4] = 0x40;
        let reader = FakeReader::new(vec![(RECORD, other)]);
        assert_eq!(upgrade_fingerprint(&reader, RECORD), None);
    }
}
