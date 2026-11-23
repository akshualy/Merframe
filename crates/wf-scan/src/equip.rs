use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaTable, LuaValue};

pub(crate) const MANAGER_SCRIPT: &str = "OnConfirmEquipProjection";
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
    let relic_type = local.text(reader, FULL_NAME)?;
    Some(RelicPick { handle, relic_type })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelicPicker {
    Closed,
    Open(Option<RelicPick>),
}

impl RelicPicker {
    pub fn read<R: MemoryReader + ?Sized>(
        reader: &R,
        state: &LuaState,
        script: &mut Option<LuaTable>,
    ) -> Self {
        state
            .cached_script(reader, MANAGER_SCRIPT, script)
            .map_or(Self::Closed, |manager| {
                let locals = manager.locals(reader);
                Self::Open(locals.into_iter().find_map(|local| pick(reader, local)))
            })
    }
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
    use crate::lua::tests::{Heap, located, read_with};

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

    #[test]
    fn selected_relic_in_open_picker() {
        let reader = game(true, true, None);
        assert_eq!(
            read_with(&reader, RelicPicker::read),
            RelicPicker::Open(Some(RelicPick {
                handle: HANDLE,
                relic_type: RELIC.to_owned(),
            }))
        );
        assert_eq!(confirmed_relic(&reader, &located(&reader)), None);
    }

    #[test]
    fn nothing_selected_yet() {
        let reader = game(true, false, None);
        assert_eq!(
            read_with(&reader, RelicPicker::read),
            RelicPicker::Open(None)
        );
    }

    #[test]
    fn confirmed_handle_outlives_picker() {
        let reader = game(false, false, Some(HANDLE));
        assert_eq!(read_with(&reader, RelicPicker::read), RelicPicker::Closed);
        assert_eq!(confirmed_relic(&reader, &located(&reader)), Some(HANDLE));
    }
}
