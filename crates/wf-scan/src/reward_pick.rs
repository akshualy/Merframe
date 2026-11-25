use wf_mem::MemoryReader;

use crate::bundle::{FULL_NAME, reward_elements};
use crate::lua::{LuaState, LuaTable, LuaValue};

const QUANTITY: &str = "QuantityMultiplier";
const PLAYER: &str = "mPlayerInfo";
const PLAYER_ID: &str = "Id";
const PLAYER_IS_LOCAL: &str = "IsLocal";
const PLAYER_CHOSEN_OWNER: &str = "ChosenRewardOwner";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickedRelicReward {
    pub item_type: String,
    pub count: u32,
}

pub fn picked_relic_reward<R: MemoryReader + ?Sized>(
    reader: &R,
    state: &LuaState,
    script: &mut Option<LuaTable>,
) -> Option<PickedRelicReward> {
    let players: Vec<(LuaTable, LuaTable)> = reward_elements(reader, state, script)
        .into_iter()
        .filter_map(|element| Some((element, element.field(reader, PLAYER)?.table()?)))
        .collect();
    let (_, local) = players.iter().find(|(_, player)| {
        player.field(reader, PLAYER_IS_LOCAL) == Some(LuaValue::Boolean(true))
    })?;
    let own_id = local.text(reader, PLAYER_ID)?;
    let owner = local
        .filled_text(reader, PLAYER_CHOSEN_OWNER)
        .unwrap_or(own_id);
    let (element, _) = players
        .iter()
        .find(|(_, player)| player.text(reader, PLAYER_ID).as_ref() == Some(&owner))?;

    Some(PickedRelicReward {
        item_type: element.text(reader, FULL_NAME)?,
        count: element
            .field(reader, QUANTITY)
            .and_then(LuaValue::whole)
            .unwrap_or(1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{ELEMENTS, REWARD_SCRIPT};
    use crate::lua::tests::{Heap, read_with};

    const FORMA: &str = "/Lotus/Types/Recipes/Components/FormaBlueprint";
    const VELOX: &str = "/Lotus/Types/Recipes/Weapons/VeloxPrimeBlueprint";

    fn pick(own_choice: &str, forma_count: Option<f32>) -> Option<PickedRelicReward> {
        let reader = Heap::with_scripts(|heap| {
            let players = [
                ("own", true, own_choice, VELOX, None),
                ("mate", false, "", FORMA, forma_count),
            ];
            let elements: Vec<u64> = players
                .into_iter()
                .map(|(id, local, chosen, path, count)| {
                    let player = [
                        heap.text_field(PLAYER_ID, id),
                        heap.flag_field(PLAYER_IS_LOCAL, local),
                        heap.text_field(PLAYER_CHOSEN_OWNER, chosen),
                    ];
                    let player = heap.table(&player);
                    let mut element = vec![
                        heap.table_field(PLAYER, player),
                        heap.text_field(FULL_NAME, path),
                    ];
                    element.extend(count.map(|count| heap.number_field(QUANTITY, count)));
                    heap.table(&element)
                })
                .collect();
            let grid = heap.grid(ELEMENTS, &elements);
            let rewards = heap.function_field(REWARD_SCRIPT, &[grid]);
            vec![heap.table(&[rewards])]
        });
        read_with(&reader, picked_relic_reward)
    }

    #[test]
    fn own_reward_until_another_is_chosen() {
        let picked = |item_type: &str, count| {
            Some(PickedRelicReward {
                item_type: item_type.to_owned(),
                count,
            })
        };
        assert_eq!(pick("", None), picked(VELOX, 1));
        assert_eq!(pick("mate", None), picked(FORMA, 1));
        assert_eq!(pick("mate", Some(2.0)), picked(FORMA, 2));
    }
}
