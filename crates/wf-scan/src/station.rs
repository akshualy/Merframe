use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaTable};

const CHOICE: &str = "OmegaRerollChoice";
const ITEM_ID: &str = "Id";
const FINGERPRINT: &str = "Fingerprint";
const STATION_SCRIPT: &str = "OnOmegaRerollCommitted";
const CLIP: &str = "ClipName";
const SHOWN_CLIP: &str = "Choice1";
const OFFERED_CLIP: &str = "Choice2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StationRiven {
    pub item_id: String,
    pub shown: String,
    pub offered: Option<String>,
}

fn clip<R: MemoryReader + ?Sized>(reader: &R, cards: &[LuaTable], name: &str) -> Option<String> {
    let card = cards
        .iter()
        .find(|card| card.text(reader, CLIP).as_deref() == Some(name))?;
    card.text(reader, FINGERPRINT)
}

fn cards<R: MemoryReader + ?Sized>(
    reader: &R,
    state: &LuaState,
    script: &mut Option<LuaTable>,
) -> Option<(String, Option<String>)> {
    state
        .cached_script(reader, STATION_SCRIPT, script)?
        .locals(reader)
        .into_iter()
        .find_map(|cards| {
            let cards = cards.tables(reader);
            Some((
                clip(reader, &cards, SHOWN_CLIP)?,
                clip(reader, &cards, OFFERED_CLIP),
            ))
        })
}

impl StationRiven {
    pub fn read<R: MemoryReader + ?Sized>(
        reader: &R,
        state: &LuaState,
        script: &mut Option<LuaTable>,
    ) -> Option<Self> {
        let (shown, offered) = cards(reader, state, script)?;
        let choice = state.shared(reader)?.field(reader, CHOICE)?.table()?;
        Some(Self {
            item_id: choice.text(reader, ITEM_ID)?,
            shown,
            offered,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::{Heap, read_with};

    fn choice(heap: &mut Heap) {
        let choice = [
            heap.text_field(ITEM_ID, "67b8145f6fae460f420380bd"),
            heap.text_field(FINGERPRINT, "{\"rerolls\":1}"),
        ];
        let choice = heap.table(&choice);
        let choice = heap.table_field(CHOICE, choice);
        heap.shared.push(choice);
    }

    fn station_script(heap: &mut Heap, cards: &[(&str, &str)]) -> u64 {
        let cards: Vec<u64> = cards
            .iter()
            .map(|(clip, fingerprint)| {
                let card = [
                    heap.text_field(CLIP, clip),
                    heap.text_field(FINGERPRINT, fingerprint),
                ];
                heap.table(&card)
            })
            .collect();
        let cards = heap.list(&cards);
        let other = heap.table(&[]);
        let committed = heap.function_field(STATION_SCRIPT, &[other, cards]);
        heap.table(&[committed])
    }

    fn station(cards: &[(&str, &str)]) -> FakeReader {
        Heap::with_scripts(|heap| {
            choice(heap);
            vec![station_script(heap, cards)]
        })
    }

    #[test]
    fn cycled_riven_with_both_rolls() {
        let reader = station(&[
            ("Choice1", "{\"rerolls\":3}"),
            ("Choice2", "{\"rerolls\":3,\"buffs\":[]}"),
        ]);
        assert_eq!(
            read_with(&reader, StationRiven::read),
            Some(StationRiven {
                item_id: "67b8145f6fae460f420380bd".to_owned(),
                shown: "{\"rerolls\":3}".to_owned(),
                offered: Some("{\"rerolls\":3,\"buffs\":[]}".to_owned()),
            })
        );
    }

    #[test]
    fn selected_riven_uncycled() {
        let reader = station(&[("Choice1", "{\"rerolls\":2}")]);
        let riven = read_with(&reader, StationRiven::read).unwrap();
        assert_eq!(riven.shown, "{\"rerolls\":2}");
        assert_eq!(riven.offered, None);
    }

    #[test]
    fn closed_station_stale_choice() {
        let reader = Heap::with_scripts(|heap| {
            let choice = [heap.text_field(ITEM_ID, "67b8145f6fae460f420380bd")];
            let choice = heap.table(&choice);
            let choice = heap.table_field(CHOICE, choice);
            heap.shared.push(choice);
            vec![heap.table(&[])]
        });
        assert_eq!(read_with(&reader, StationRiven::read), None);
    }
}
