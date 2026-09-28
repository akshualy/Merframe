use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaTable, LuaValue};

const TRADE_SCRIPT: &str = "AcceptTrade";
const ELEMENTS: &str = "mElements";
const CLIP: &str = "mClipName";
const SIDE_CLIPS: [&str; 2] = ["PlayerTradeMenu.", "PartnerTradeMenu."];
const NAME: &str = "Name";
const FULL_NAME: &str = "FullName";
const COUNT: &str = "Count";
const FINGERPRINT: &str = "Fingerprint";
const PARTNER: &str = "PLAYER_NAME";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeSlot {
    pub name: String,
    pub item_type: Option<String>,
    pub count: u32,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeScreen {
    pub partner: Option<String>,
    pub offered: Vec<TradeSlot>,
    pub received: Vec<TradeSlot>,
}

impl TradeScreen {
    pub fn read<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> Option<Self> {
        let mut partner = None;
        let mut sides: [Option<Vec<TradeSlot>>; 2] = [None, None];
        for local in state.script(reader, TRADE_SCRIPT)?.locals(reader) {
            if let Some(name) = local.field(reader, PARTNER).and_then(LuaValue::text) {
                partner = Some(name);
                continue;
            }
            let Some(elements) = local.field(reader, ELEMENTS).and_then(LuaValue::table) else {
                continue;
            };
            let elements: Vec<LuaTable> = elements
                .items(reader)
                .into_iter()
                .filter_map(LuaValue::table)
                .collect();
            let Some(clip) = elements
                .first()
                .and_then(|element| element.field(reader, CLIP)?.text())
            else {
                continue;
            };
            let Some(side) = SIDE_CLIPS
                .iter()
                .position(|prefix| clip.starts_with(prefix))
            else {
                continue;
            };
            sides[side] = Some(
                elements
                    .into_iter()
                    .filter_map(|element| slot(reader, element))
                    .collect(),
            );
        }
        let [offered, received] = sides;
        Some(Self {
            partner,
            offered: offered?,
            received: received?,
        })
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole(number: f32) -> Option<u32> {
    (number >= 0.0 && number.fract() == 0.0).then_some(number as u32)
}

fn slot<R: MemoryReader + ?Sized>(reader: &R, element: LuaTable) -> Option<TradeSlot> {
    Some(TradeSlot {
        name: element.field(reader, NAME)?.text()?,
        item_type: element.field(reader, FULL_NAME).and_then(LuaValue::text),
        count: whole(element.field(reader, COUNT)?.number()?)?,
        fingerprint: element
            .field(reader, FINGERPRINT)
            .and_then(LuaValue::text)
            .filter(|fingerprint| !fingerprint.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::Heap;

    const ARCANE: &str = "/Lotus/Upgrades/CosmeticEnhancers/Offensive/OrbsOnResidualContact";

    type Entry<'a> = (&'a str, &'a [(&'a str, &'a str)], f32);

    fn screen(own: &[Entry<'_>], partner: &[Entry<'_>]) -> FakeReader {
        Heap::with_scripts(|heap| {
            let mut grids = Vec::new();
            for (prefix, entries) in SIDE_CLIPS.into_iter().zip([own, partner]) {
                let mut elements = Vec::new();
                for (index, (name, texts, count)) in entries.iter().enumerate() {
                    let clip = format!(
                        "{prefix}MenuEntry{}",
                        if index == 0 {
                            String::new()
                        } else {
                            (index + 1).to_string()
                        }
                    );
                    let mut fields = vec![heap.text_field(CLIP, &clip)];
                    if !name.is_empty() {
                        fields.push(heap.text_field(NAME, name));
                        fields.push(heap.number_field(COUNT, *count));
                        for (key, text) in *texts {
                            fields.push(heap.text_field(key, text));
                        }
                    }
                    elements.push(heap.table(&fields));
                }
                let elements = heap.list(&elements);
                let grid = [heap.table_field(ELEMENTS, elements)];
                grids.push(heap.table(&grid));
            }
            let args = [heap.text_field(PARTNER, "TestSquadA\u{e000}")];
            grids.push(heap.table(&args));
            let accept = heap.function_field(TRADE_SCRIPT, &grids);
            vec![heap.table(&[accept])]
        })
    }

    fn read(reader: &FakeReader) -> Option<TradeScreen> {
        TradeScreen::read(reader, &LuaState::locate(reader).unwrap()?)
    }

    #[test]
    fn arcanes_for_platinum() {
        let reader = screen(
            &[("", &[], 0.0), ("Platinum", &[], 84.0)],
            &[
                (
                    "Theorem Contagion",
                    &[(FULL_NAME, ARCANE), (FINGERPRINT, "{\"lvl\":3}")],
                    1.0,
                ),
                (
                    "Theorem Contagion",
                    &[(FULL_NAME, ARCANE), (FINGERPRINT, "")],
                    1.0,
                ),
                ("", &[], 0.0),
            ],
        );
        assert_eq!(
            read(&reader),
            Some(TradeScreen {
                partner: Some(String::from("TestSquadA\u{e000}")),
                offered: vec![TradeSlot {
                    name: String::from("Platinum"),
                    item_type: None,
                    count: 84,
                    fingerprint: None,
                }],
                received: vec![
                    TradeSlot {
                        name: String::from("Theorem Contagion"),
                        item_type: Some(ARCANE.to_owned()),
                        count: 1,
                        fingerprint: Some(String::from("{\"lvl\":3}")),
                    },
                    TradeSlot {
                        name: String::from("Theorem Contagion"),
                        item_type: Some(ARCANE.to_owned()),
                        count: 1,
                        fingerprint: None,
                    },
                ],
            })
        );
    }

    #[test]
    fn counts_are_whole_numbers() {
        assert_eq!(whole(84.0), Some(84));
        assert_eq!(whole(0.0), Some(0));
        assert_eq!(whole(1.5), None);
        assert_eq!(whole(-1.0), None);
    }

    #[test]
    fn one_grid_is_not_a_trade() {
        let reader = Heap::with_scripts(|heap| {
            let element = [heap.text_field(CLIP, "PlayerTradeMenu.MenuEntry")];
            let element = heap.table(&element);
            let elements = heap.list(&[element]);
            let grid = [heap.table_field(ELEMENTS, elements)];
            let grid = heap.table(&grid);
            let accept = heap.function_field(TRADE_SCRIPT, &[grid]);
            vec![heap.table(&[accept])]
        });
        assert_eq!(read(&reader), None);
    }

    #[test]
    fn screen_closed() {
        assert_eq!(
            read(&Heap::with_scripts(|heap| vec![heap.table(&[])])),
            None
        );
    }
}
