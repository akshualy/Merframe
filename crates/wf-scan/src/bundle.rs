use wf_mem::MemoryReader;

use crate::Result;
use crate::lua::{LuaState, LuaTable, LuaValue};

const REWARD_SCRIPT: &str = "OnVoidRewards";
const ELEMENTS: &str = "mElements";
const CLIP: &str = "mClipName";
const CLIP_NAME: &str = "RewardList.Item";
const FULL_NAME: &str = "FullName";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RewardTile {
    pub slot: u8,
    pub item_type: String,
    pub store_item: String,
}

fn clip_slot(clip: &str) -> Option<u8> {
    match clip.strip_prefix(CLIP_NAME)?.as_bytes() {
        [] => Some(1),
        [digit] if digit.is_ascii_digit() => Some(digit - b'0'),
        _ => None,
    }
}

fn tile<R: MemoryReader + ?Sized>(reader: &R, element: LuaTable) -> Option<RewardTile> {
    let slot = clip_slot(&element.field(reader, CLIP)?.text()?)?;
    let item_type = element.field(reader, FULL_NAME)?.text()?;
    let store_item = format!("/Lotus/StoreItems/{}", item_type.strip_prefix("/Lotus/")?);
    Some(RewardTile {
        slot,
        item_type,
        store_item,
    })
}

fn reward_tiles<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> Vec<RewardTile> {
    let Some(script) = state.script(reader, REWARD_SCRIPT) else {
        return Vec::new();
    };
    let mut tiles: Vec<RewardTile> = script
        .locals(reader)
        .into_iter()
        .filter_map(|local| local.field(reader, ELEMENTS)?.table())
        .flat_map(|elements| elements.items(reader))
        .filter_map(LuaValue::table)
        .filter_map(|element| tile(reader, element))
        .collect();
    tiles.sort_unstable();
    tiles.dedup();
    tiles
}

pub fn reward_screen<R: MemoryReader + ?Sized>(reader: &R) -> Result<Vec<RewardTile>> {
    Ok(LuaState::locate(reader)?
        .map(|state| reward_tiles(reader, &state))
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua::tests::Heap;

    fn scan(tiles: &[(&str, &str)]) -> Vec<RewardTile> {
        let reader = Heap::with_scripts(|heap| {
            let elements: Vec<u64> = tiles
                .iter()
                .map(|(clip, path)| {
                    let element = [
                        heap.text_field(CLIP, clip),
                        heap.text_field(FULL_NAME, path),
                    ];
                    heap.table(&element)
                })
                .collect();
            let elements = heap.list(&elements);
            let grid = [heap.table_field(ELEMENTS, elements)];
            let grid = heap.table(&grid);
            let rewards = heap.function_field(REWARD_SCRIPT, &[grid]);
            vec![heap.table(&[rewards])]
        });
        reward_screen(&reader).unwrap()
    }

    fn reward(slot: u8, path: &str) -> RewardTile {
        RewardTile {
            slot,
            item_type: format!("/Lotus/{path}"),
            store_item: format!("/Lotus/StoreItems/{path}"),
        }
    }

    #[test]
    fn four_tiles_in_screen_order() {
        let tiles = scan(&[
            (
                "RewardList.Item3",
                "/Lotus/Types/Recipes/Components/FormaBlueprint",
            ),
            (
                "RewardList.Item1",
                "/Lotus/Types/Recipes/Weapons/VeloxPrimeBlueprint",
            ),
            (
                "RewardList.Item4",
                "/Lotus/Types/Recipes/Components/FormaBlueprint",
            ),
            (
                "RewardList.Item2",
                "/Lotus/Types/Recipes/WarframeRecipes/VorunaPrimeHelmetBlueprint",
            ),
        ]);
        assert_eq!(
            tiles,
            vec![
                reward(1, "Types/Recipes/Weapons/VeloxPrimeBlueprint"),
                reward(
                    2,
                    "Types/Recipes/WarframeRecipes/VorunaPrimeHelmetBlueprint"
                ),
                reward(3, "Types/Recipes/Components/FormaBlueprint"),
                reward(4, "Types/Recipes/Components/FormaBlueprint"),
            ]
        );
    }

    #[test]
    fn solo_screen() {
        assert_eq!(
            scan(&[(
                "RewardList.Item",
                "/Lotus/Types/Recipes/Components/FormaBlueprint"
            )]),
            vec![reward(1, "Types/Recipes/Components/FormaBlueprint")]
        );
    }

    #[test]
    fn element_of_another_list() {
        assert_eq!(
            scan(&[(
                "InventoryGrid.Item2",
                "/Lotus/Types/Recipes/Components/FormaBlueprint"
            )]),
            Vec::new()
        );
    }

    #[test]
    fn longer_clip_name() {
        assert_eq!(
            scan(&[(
                "RewardList.Item12",
                "/Lotus/Types/Recipes/Components/FormaBlueprint"
            )]),
            Vec::new()
        );
    }

    #[test]
    fn screen_closed() {
        assert_eq!(scan(&[]), Vec::new());
    }
}
