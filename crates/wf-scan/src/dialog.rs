use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaTable};

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
    pub fn read<R: MemoryReader + ?Sized>(
        reader: &R,
        state: &LuaState,
        script: &mut Option<LuaTable>,
    ) -> Option<Self> {
        state
            .cached_script(reader, DIALOG_SCRIPT, script)?
            .locals(reader)
            .into_iter()
            .find_map(|local| {
                let item = local.field(reader, ITEM)?.table()?;
                Some(Self {
                    fingerprint: item.filled_text(reader, FINGERPRINT)?,
                    mod_type: item
                        .field(reader, STORE_ITEM)?
                        .table()?
                        .text(reader, FULL_NAME)?,
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::{Heap, read_with};

    const MOD_TYPE: &str = "/Lotus/Upgrades/Mods/Randomized/PlayerMeleeWeaponRandomModRare";
    const ROLL: &str =
        "{\"compat\":\"/Lotus/Weapons/Corpus/Melee/KickAndPunch/KickPunchWeapon\",\"rerolls\":20}";

    fn dialog_script(heap: &mut Heap, fingerprint: &str) -> u64 {
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
        heap.table(&[gift])
    }

    fn dialog(fingerprint: &str) -> FakeReader {
        Heap::with_scripts(|heap| vec![dialog_script(heap, fingerprint)])
    }

    #[test]
    fn linked_riven() {
        assert_eq!(
            read_with(&dialog(ROLL), DialogRiven::read),
            Some(DialogRiven {
                mod_type: MOD_TYPE.to_owned(),
                fingerprint: ROLL.to_owned(),
            })
        );
    }

    #[test]
    fn missing_fingerprint() {
        assert_eq!(read_with(&dialog(""), DialogRiven::read), None);
    }

    #[test]
    fn dialog_closed() {
        let reader = Heap::with_scripts(|heap| vec![heap.table(&[])]);
        assert_eq!(read_with(&reader, DialogRiven::read), None);
    }
}
