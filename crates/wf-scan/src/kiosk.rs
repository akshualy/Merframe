use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaTable, LuaValue};

const KIOSK_SCRIPT: &str = "SetOnMarkedToSellFunction";
const CLIP: &str = "mClipName";
const GRID_CLIP: &str = "InventoryGrid.InventoryItem";
const SELL_LIST_CLIP: &str = "SellList.List.Item";
const GRID_ELEMENTS: &str = "mUnfilteredElements";
const SELL_LIST_ELEMENTS: &str = "mElements";
const NAME: &str = "Name";
const FULL_NAME: &str = "FullName";
const COUNT: &str = "Count";
const PRICE: &str = "SellingPrice";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KioskItem {
    pub item_type: String,
    pub name: String,
    pub count: u32,
    pub ducats: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DucatKiosk {
    pub owned: Vec<KioskItem>,
    pub marked: Vec<KioskItem>,
}

impl DucatKiosk {
    pub fn read<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> Option<Self> {
        let mut owned = None;
        let mut marked = None;
        for local in state.script(reader, KIOSK_SCRIPT)?.locals(reader) {
            match local
                .field(reader, CLIP)
                .and_then(LuaValue::text)
                .as_deref()
            {
                Some(GRID_CLIP) if owned.is_none() => {
                    owned = items(reader, local, GRID_ELEMENTS);
                }
                Some(SELL_LIST_CLIP) if marked.is_none() => {
                    marked = items(reader, local, SELL_LIST_ELEMENTS);
                }
                _ => {}
            }
        }
        Some(Self {
            owned: owned.filter(|owned| !owned.is_empty())?,
            marked: marked?,
        })
    }
}

fn items<R: MemoryReader + ?Sized>(
    reader: &R,
    list: LuaTable,
    elements: &str,
) -> Option<Vec<KioskItem>> {
    let elements = list.field(reader, elements)?.table()?;
    Some(
        elements
            .items(reader)
            .into_iter()
            .filter_map(LuaValue::table)
            .filter_map(|element| item(reader, element))
            .collect(),
    )
}

fn item<R: MemoryReader + ?Sized>(reader: &R, element: LuaTable) -> Option<KioskItem> {
    Some(KioskItem {
        item_type: element.field(reader, FULL_NAME)?.text()?,
        name: element.field(reader, NAME)?.text()?,
        count: element.field(reader, COUNT)?.whole()?,
        ducats: element.field(reader, PRICE)?.whole()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::Heap;

    const STOCK: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock";
    const LINK: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/AkboltoPrimeLink";

    type Entry<'a> = (&'a str, &'a str, u16, u16);

    fn list(heap: &mut Heap, clip: &str, elements: &str, entries: &[Entry<'_>]) -> (u64, Vec<u64>) {
        let mut tables: Vec<u64> = entries
            .iter()
            .map(|(item_type, name, count, ducats)| {
                let fields = [
                    heap.text_field(CLIP, clip),
                    heap.text_field(FULL_NAME, item_type),
                    heap.text_field(NAME, name),
                    heap.number_field(COUNT, f32::from(*count)),
                    heap.number_field(PRICE, f32::from(*ducats)),
                ];
                heap.table(&fields)
            })
            .collect();
        let filler = [heap.number_field("Id", 40.0)];
        tables.push(heap.table(&filler));
        let array = heap.list(&tables);
        let fields = [
            heap.text_field(CLIP, clip),
            heap.table_field(elements, array),
        ];
        (heap.table(&fields), tables)
    }

    fn kiosk_with(owned: &[Entry<'_>], marked: &[Entry<'_>], focused_entry: bool) -> FakeReader {
        Heap::with_scripts(|heap| {
            let (grid, _) = list(heap, GRID_CLIP, GRID_ELEMENTS, owned);
            let (sell_list, entries) = list(heap, SELL_LIST_CLIP, SELL_LIST_ELEMENTS, marked);
            let mut locals = vec![grid, sell_list];
            if focused_entry {
                locals.insert(0, entries[0]);
            }
            let script = [heap.function_field(KIOSK_SCRIPT, &locals)];
            vec![heap.table(&script)]
        })
    }

    fn kiosk(owned: &[Entry<'_>], marked: &[Entry<'_>]) -> FakeReader {
        kiosk_with(owned, marked, false)
    }

    fn read(reader: &FakeReader) -> Option<DucatKiosk> {
        DucatKiosk::read(reader, &LuaState::locate(reader).unwrap()?)
    }

    fn expected((item_type, name, count, ducats): Entry<'_>) -> KioskItem {
        KioskItem {
            item_type: item_type.to_owned(),
            name: name.to_owned(),
            count: u32::from(count),
            ducats: u32::from(ducats),
        }
    }

    #[test]
    fn marked_items() {
        let stock = (STOCK, "Braton Prime Stock", 7, 15);
        let link = (LINK, "Akbolto Prime Link", 2, 45);
        let marked = (STOCK, "Braton Prime Stock", 3, 15);
        assert_eq!(
            read(&kiosk(&[stock, link], &[marked])),
            Some(DucatKiosk {
                owned: vec![expected(stock), expected(link)],
                marked: vec![expected(marked)],
            })
        );
    }

    #[test]
    fn focused_entry_shares_the_list_clip() {
        let stock = (STOCK, "Braton Prime Stock", 7, 15);
        let marked = (STOCK, "Braton Prime Stock", 3, 15);
        assert_eq!(
            read(&kiosk_with(&[stock], &[marked], true)).map(|kiosk| kiosk.marked),
            Some(vec![expected(marked)])
        );
    }

    #[test]
    fn nothing_marked() {
        let link = (LINK, "Akbolto Prime Link", 2, 45);
        assert_eq!(
            read(&kiosk(&[link], &[])).map(|kiosk| kiosk.marked),
            Some(Vec::new())
        );
    }

    #[test]
    fn empty_grid_is_not_read() {
        assert_eq!(read(&kiosk(&[], &[])), None);
    }

    #[test]
    fn kiosk_closed() {
        assert_eq!(
            read(&Heap::with_scripts(|heap| vec![heap.table(&[])])),
            None
        );
    }
}
