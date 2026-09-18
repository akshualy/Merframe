use wf_mem::MemoryReader;

use crate::Result;
use crate::lua::{LuaState, LuaTable, LuaValue};

const VOID_TEAR: &str = "mVoidTear";
const MISSION_TIER: &str = "MissionTier";
const SECTOR_NAME: &str = "SectorName";
const TIERS: [&str; 6] = ["VoidT1", "VoidT2", "VoidT3", "VoidT4", "VoidT5", "VoidT6"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelicPickerNode {
    pub node: String,
    pub void_tier: &'static str,
}

fn void_tear<R: MemoryReader + ?Sized>(reader: &R, script: LuaTable) -> Option<RelicPickerNode> {
    let tear = script.field(reader, VOID_TEAR)?.table()?;
    let tier = tear.field(reader, MISSION_TIER)?.text()?;
    let void_tier = TIERS.into_iter().find(|t| *t == tier)?;
    let node = tear
        .field(reader, SECTOR_NAME)
        .and_then(LuaValue::text)
        .filter(|node| !node.is_empty())?;
    Some(RelicPickerNode { node, void_tier })
}

#[derive(Debug, Default)]
pub struct RelicPicker {
    state: Option<LuaState>,
}

impl RelicPicker {
    pub fn find<R: MemoryReader + ?Sized>(
        &mut self,
        reader: &R,
    ) -> Result<Option<RelicPickerNode>> {
        if !self.state.is_some_and(|state| state.holds(reader)) {
            self.state = LuaState::locate(reader)?;
        }
        Ok(self.state.and_then(|state| {
            state
                .scripts(reader)
                .into_iter()
                .find_map(|script| void_tear(reader, script))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::{HEAP, Heap, IMAGE};

    fn star_chart(tears: &[(&str, &str)]) -> FakeReader {
        Heap::with_scripts(|heap| {
            tears
                .iter()
                .map(|(node, tier)| {
                    let fields = [
                        heap.text_field(SECTOR_NAME, node),
                        heap.text_field(MISSION_TIER, tier),
                    ];
                    let tear = heap.table(&fields);
                    let field = heap.table_field(VOID_TEAR, tear);
                    heap.table(&[field])
                })
                .collect()
        })
    }

    fn scan(tears: &[(&str, &str)]) -> Option<RelicPickerNode> {
        RelicPicker::default().find(&star_chart(tears)).unwrap()
    }

    #[test]
    fn selected_node_and_tier() {
        assert_eq!(
            scan(&[("SolNode61", "VoidT2")]),
            Some(RelicPickerNode {
                node: "SolNode61".to_string(),
                void_tier: "VoidT2",
            })
        );
    }

    #[test]
    fn skips_empty_template() {
        assert_eq!(
            scan(&[("", ""), ("SettlementNode14", "VoidT5")]),
            Some(RelicPickerNode {
                node: "SettlementNode14".to_string(),
                void_tier: "VoidT5",
            })
        );
    }

    #[test]
    fn unknown_tier() {
        assert_eq!(scan(&[("SolNode61", "VoidT9")]), None);
    }

    #[test]
    fn no_star_chart_script() {
        assert_eq!(scan(&[]), None);
    }

    #[test]
    fn state_relocated_for_new_process() {
        let mut picker = RelicPicker::default();
        let lith = RelicPickerNode {
            node: "SolNode61".to_string(),
            void_tier: "VoidT1",
        };
        let before = star_chart(&[("SolNode61", "VoidT1")]);
        assert_eq!(picker.find(&before).unwrap(), Some(lith));
        let after =
            FakeReader::with_image((IMAGE, vec![0u8; 0x100]), vec![(HEAP, vec![0u8; 0x100])]);
        assert_eq!(picker.find(&after).unwrap(), None);
    }
}
