use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaValue};

const DIALOG_SCRIPT: &str = "OnGiftRecipient";
const ITEM: &str = "ITEM";
const FINGERPRINT: &str = "UpgradeFingerprint";
const STORE_ITEM: &str = "StoreItemInfo";
const FULL_NAME: &str = "FullName";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogRiven {
    pub mod_type: String,
    pub fingerprint: String,
}

impl DialogRiven {
    pub fn read<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> Option<Self> {
        state
            .script(reader, DIALOG_SCRIPT)?
            .locals(reader)
            .into_iter()
            .find_map(|local| {
                let item = local.field(reader, ITEM)?.table()?;
                Some(Self {
                    fingerprint: item
                        .field(reader, FINGERPRINT)
                        .and_then(LuaValue::text)
                        .filter(|fingerprint| !fingerprint.is_empty())?,
                    mod_type: item
                        .field(reader, STORE_ITEM)?
                        .table()?
                        .field(reader, FULL_NAME)?
                        .text()?,
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::Heap;

    const MOD_TYPE: &str = "/Lotus/Upgrades/Mods/Randomized/PlayerMeleeWeaponRandomModRare";
    const ROLL: &str =
        "{\"compat\":\"/Lotus/Weapons/Corpus/Melee/KickAndPunch/KickPunchWeapon\",\"rerolls\":20}";

    fn dialog(fingerprint: &str) -> FakeReader {
        Heap::with_scripts(|heap| {
            let info = [heap.text_field(FULL_NAME, MOD_TYPE)];
            let info = heap.table(&info);
            let item = [
                heap.text_field(FINGERPRINT, fingerprint),
                heap.table_field(STORE_ITEM, info),
            ];
            let item = heap.table(&item);
            let item = [heap.table_field(ITEM, item)];
            let local = heap.table(&item);
            let gift = heap.function_field(DIALOG_SCRIPT, &[local]);
            vec![heap.table(&[gift])]
        })
    }

    fn read(reader: &FakeReader) -> Option<DialogRiven> {
        DialogRiven::read(reader, &LuaState::locate(reader).unwrap()?)
    }

    #[test]
    fn linked_riven() {
        assert_eq!(
            read(&dialog(ROLL)),
            Some(DialogRiven {
                mod_type: MOD_TYPE.to_owned(),
                fingerprint: ROLL.to_owned(),
            })
        );
    }

    #[test]
    fn missing_fingerprint() {
        assert_eq!(read(&dialog("")), None);
    }

    #[test]
    fn dialog_closed() {
        assert_eq!(
            read(&Heap::with_scripts(|heap| vec![heap.table(&[])])),
            None
        );
    }
}
