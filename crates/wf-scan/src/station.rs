use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaTable, LuaValue};

const SHARED_TABLE: &str = "_T";
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

fn text<R: MemoryReader + ?Sized>(reader: &R, table: LuaTable, name: &str) -> Option<String> {
    table.field(reader, name)?.text()
}

fn clip<R: MemoryReader + ?Sized>(reader: &R, cards: &[LuaTable], name: &str) -> Option<String> {
    let card = cards
        .iter()
        .find(|card| text(reader, **card, CLIP).as_deref() == Some(name))?;
    text(reader, *card, FINGERPRINT)
}

fn cards<R: MemoryReader + ?Sized>(
    reader: &R,
    state: &LuaState,
) -> Option<(String, Option<String>)> {
    state
        .script(reader, STATION_SCRIPT)?
        .locals(reader)
        .into_iter()
        .find_map(|cards| {
            let cards: Vec<LuaTable> = cards
                .items(reader)
                .into_iter()
                .filter_map(LuaValue::table)
                .collect();
            Some((
                clip(reader, &cards, SHOWN_CLIP)?,
                clip(reader, &cards, OFFERED_CLIP),
            ))
        })
}

impl StationRiven {
    pub fn read<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> Option<Self> {
        let (shown, offered) = cards(reader, state)?;
        let choice = state
            .global(reader, SHARED_TABLE)?
            .table()?
            .field(reader, CHOICE)?
            .table()?;
        Some(Self {
            item_id: text(reader, choice, ITEM_ID)?,
            shown,
            offered,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::Heap;

    fn station(cards: &[(&str, &str)]) -> FakeReader {
        Heap::with_scripts(|heap| {
            let choice = [
                heap.text_field(ITEM_ID, "67b8145f6fae460f420380bd"),
                heap.text_field(FINGERPRINT, "{\"rerolls\":1}"),
            ];
            let choice = heap.table(&choice);
            let choice = heap.table_field(CHOICE, choice);
            heap.shared.push(choice);
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
            vec![heap.table(&[committed])]
        })
    }

    fn read(reader: &FakeReader) -> Option<StationRiven> {
        StationRiven::read(reader, &LuaState::locate(reader).unwrap()?)
    }

    #[test]
    fn cycled_riven_with_both_rolls() {
        let reader = station(&[
            ("Choice1", "{\"rerolls\":3}"),
            ("Choice2", "{\"rerolls\":3,\"buffs\":[]}"),
        ]);
        assert_eq!(
            read(&reader),
            Some(StationRiven {
                item_id: "67b8145f6fae460f420380bd".to_owned(),
                shown: "{\"rerolls\":3}".to_owned(),
                offered: Some("{\"rerolls\":3,\"buffs\":[]}".to_owned()),
            })
        );
    }

    #[test]
    fn selected_riven_uncycled() {
        let riven = read(&station(&[("Choice1", "{\"rerolls\":2}")])).unwrap();
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
        assert_eq!(read(&reader), None);
    }
}
