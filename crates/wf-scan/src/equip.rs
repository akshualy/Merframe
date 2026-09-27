use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaTable, LuaValue};

const MANAGER_SCRIPT: &str = "OnConfirmEquipProjection";
const FULL_NAME: &str = "FullName";
const TYPE: &str = "Type";
const CONFIRMED_RELIC: &str = "ThemedProjectionMgr_PrevProj";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelicPick {
    pub handle: u64,
    pub relic_type: String,
}

fn pick<R: MemoryReader + ?Sized>(reader: &R, local: LuaTable) -> Option<RelicPick> {
    let handle = local.field(reader, TYPE)?.userdata()?;
    let relic_type = local.field(reader, FULL_NAME)?.text()?;
    Some(RelicPick { handle, relic_type })
}

pub fn relic_pick<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> Option<RelicPick> {
    state
        .script(reader, MANAGER_SCRIPT)?
        .locals(reader)
        .into_iter()
        .find_map(|local| pick(reader, local))
}

pub fn relic_picker_open<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> bool {
    state.script(reader, MANAGER_SCRIPT).is_some()
}

pub fn confirmed_relic<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> Option<u64> {
    state
        .global(reader, CONFIRMED_RELIC)
        .and_then(LuaValue::userdata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::Heap;

    const RELIC: &str = "/Lotus/Types/Game/Projections/T1VoidProjectionCalibanPrimeFBronze";
    const HANDLE: u64 = 0xc784_1df0;

    fn game(picker_open: bool, selected: bool, confirmed: Option<u64>) -> FakeReader {
        Heap::with_scripts(|heap| {
            if let Some(handle) = confirmed {
                let field = heap.userdata_field(CONFIRMED_RELIC, handle);
                heap.global(field);
            }
            if !picker_open {
                return vec![heap.table(&[])];
            }
            let element = if selected {
                let fields = [
                    heap.text_field(FULL_NAME, RELIC),
                    heap.userdata_field(TYPE, HANDLE),
                ];
                heap.table(&fields)
            } else {
                heap.table(&[])
            };
            let confirm = heap.function_field(MANAGER_SCRIPT, &[element]);
            vec![heap.table(&[confirm])]
        })
    }

    fn state(reader: &FakeReader) -> LuaState {
        LuaState::locate(reader).unwrap().unwrap()
    }

    #[test]
    fn selected_relic_in_open_picker() {
        let reader = game(true, true, None);
        let lua = state(&reader);
        assert!(relic_picker_open(&reader, &lua));
        assert_eq!(
            relic_pick(&reader, &lua),
            Some(RelicPick {
                handle: HANDLE,
                relic_type: RELIC.to_owned(),
            })
        );
        assert_eq!(confirmed_relic(&reader, &lua), None);
    }

    #[test]
    fn nothing_selected_yet() {
        let reader = game(true, false, None);
        let lua = state(&reader);
        assert!(relic_picker_open(&reader, &lua));
        assert_eq!(relic_pick(&reader, &lua), None);
    }

    #[test]
    fn confirmed_handle_outlives_picker() {
        let reader = game(false, false, Some(HANDLE));
        let lua = state(&reader);
        assert!(!relic_picker_open(&reader, &lua));
        assert_eq!(relic_pick(&reader, &lua), None);
        assert_eq!(confirmed_relic(&reader, &lua), Some(HANDLE));
    }
}
