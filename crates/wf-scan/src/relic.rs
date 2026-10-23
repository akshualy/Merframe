use wf_mem::MemoryReader;

use crate::lua::{LuaState, LuaTable};

const MANAGER_SCRIPT: &str = "OnConfirmEquipProjection";
const CATEGORIES: &str = "mUnfilteredCategories";
const CATEGORY: &str = "Category";
const ELEMENTS: &str = "mUnfilteredElements";
const FULL_NAME: &str = "FullName";
const RELIC_PREFIX: &str = "/Lotus/Types/Game/Projections/T";
const ERAS: [&str; 5] = ["VoidT1", "VoidT2", "VoidT3", "VoidT4", "VoidT5"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelicScreen {
    Era(&'static str),
    AnyEra,
    Orbiter,
}

fn categories<R: MemoryReader + ?Sized>(reader: &R, grid: LuaTable) -> Option<Vec<u32>> {
    let entries = grid.field(reader, CATEGORIES)?.table()?.tables(reader);
    if entries.is_empty() {
        return None;
    }

    let mut categories: Vec<u32> = entries
        .into_iter()
        .map(|entry| entry.field(reader, CATEGORY)?.whole())
        .collect::<Option<_>>()?;
    categories.sort_unstable();
    Some(categories)
}

fn offered_era<R: MemoryReader + ?Sized>(reader: &R, grid: LuaTable) -> Option<&'static str> {
    let relic = grid
        .field(reader, ELEMENTS)?
        .table()?
        .tables(reader)
        .into_iter()
        .find_map(|element| element.text(reader, FULL_NAME))?;
    let era = relic
        .strip_prefix(RELIC_PREFIX)?
        .chars()
        .next()?
        .to_digit(10)?;
    let index = usize::try_from(era).ok()?.checked_sub(1)?;
    ERAS.get(index).copied()
}

fn screen_of<R: MemoryReader + ?Sized>(reader: &R, grid: LuaTable) -> Option<RelicScreen> {
    match categories(reader, grid)?.as_slice() {
        [1] => offered_era(reader, grid).map(RelicScreen::Era),
        [0, 1, 2, 3, 4] => Some(RelicScreen::AnyEra),
        [0, 1, 2, 3, 4, 5] => Some(RelicScreen::Orbiter),
        _ => None,
    }
}

pub fn relic_screen<R: MemoryReader + ?Sized>(reader: &R, state: &LuaState) -> Option<RelicScreen> {
    state
        .script(reader, MANAGER_SCRIPT)?
        .locals(reader)
        .into_iter()
        .find_map(|grid| screen_of(reader, grid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::lua::tests::{Heap, located};

    fn game(categories: &[u8], relics: &[&str]) -> FakeReader {
        Heap::with_scripts(|heap| {
            let entries: Vec<u64> = categories
                .iter()
                .map(|category| {
                    let field = heap.number_field(CATEGORY, f32::from(*category));
                    heap.table(&[field])
                })
                .collect();
            let elements: Vec<u64> = relics
                .iter()
                .map(|relic| {
                    let field = heap.text_field(FULL_NAME, relic);
                    heap.table(&[field])
                })
                .collect();
            let categories = heap.list(&entries);
            let elements = heap.list(&elements);
            let fields = [
                heap.table_field(CATEGORIES, categories),
                heap.table_field(ELEMENTS, elements),
            ];
            let grid = heap.table(&fields);
            let confirm = heap.function_field(MANAGER_SCRIPT, &[grid]);
            vec![heap.table(&[confirm])]
        })
    }

    fn scan(categories: &[u8], relics: &[&str]) -> Option<RelicScreen> {
        let reader = game(categories, relics);
        relic_screen(&reader, &located(&reader))
    }

    const MESO: &str = "/Lotus/Types/Game/Projections/T2VoidProjectionVorunaPrimeCGold";
    const AXI: &str = "/Lotus/Types/Game/Projections/T4VoidProjectionCalibanPrimeFBronze";
    const REQUIEM: &str = "/Lotus/Types/Game/Projections/T5VoidProjectionImmortalOmniA";

    #[test]
    fn one_era_named_by_its_relics() {
        assert_eq!(scan(&[1], &[MESO, MESO]), Some(RelicScreen::Era("VoidT2")));
        assert_eq!(scan(&[1], &[AXI]), Some(RelicScreen::Era("VoidT4")));
        assert_eq!(scan(&[1], &[REQUIEM]), Some(RelicScreen::Era("VoidT5")));
    }

    #[test]
    fn one_era_without_owned_relics() {
        assert_eq!(scan(&[1], &[]), None);
        assert_eq!(scan(&[1], &["/Lotus/Types/Items/MiscItems/Filler"]), None);
    }

    #[test]
    fn omnia_offers_every_era_but_requiem() {
        assert_eq!(
            scan(&[0, 1, 2, 3, 4], &[MESO, AXI]),
            Some(RelicScreen::AnyEra)
        );
        assert_eq!(scan(&[3, 0, 1, 4, 2], &[]), Some(RelicScreen::AnyEra));
    }

    #[test]
    fn orbiter_offers_requiem_too() {
        assert_eq!(
            scan(&[0, 1, 2, 3, 4, 5], &[MESO]),
            Some(RelicScreen::Orbiter)
        );
    }

    #[test]
    fn partial_or_unknown_lists() {
        assert_eq!(scan(&[], &[MESO]), None);
        assert_eq!(scan(&[0], &[MESO]), None);
        assert_eq!(scan(&[0, 1], &[MESO]), None);
        assert_eq!(scan(&[0, 1, 2, 3, 4, 5, 6], &[MESO]), None);
    }

    #[test]
    fn no_relic_screen() {
        let reader = Heap::with_scripts(|heap| vec![heap.table(&[])]);
        assert_eq!(relic_screen(&reader, &located(&reader)), None);
    }
}
