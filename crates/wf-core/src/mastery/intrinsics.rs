use wf_inventory::Inventory;

use super::routes::{rank_member, route};
use super::{Acquisition, CategoryTotals, Level, LevelUpRoute, MasteryGroup, MasteryItem};

pub(super) const INTRINSIC_MASTERY_XP: u64 = 1500;
const INTRINSIC_RANKS_PER_SCHOOL: u32 = 10;

pub(super) fn intrinsics(inventory: &Inventory) -> (CategoryTotals, CategoryTotals) {
    let skills = &inventory.player_skills;
    (
        CategoryTotals::new(skills.railjack(), 50),
        CategoryTotals::new(skills.duviri(), 40),
    )
}

fn intrinsic_route(
    kind: &'static str,
    label: &'static str,
    trees: &[(&'static str, u32)],
) -> LevelUpRoute {
    let mut count = 0;
    let mut members = Vec::new();
    for (name, rank) in trees {
        let left = INTRINSIC_RANKS_PER_SCHOOL.saturating_sub(*rank);
        if left == 0 {
            continue;
        }
        count += left;
        members.push(rank_member((*name).to_owned(), left, INTRINSIC_MASTERY_XP));
    }
    route(kind, label, "rank", count, members)
}

pub(super) fn routes(inventory: &Inventory) -> [LevelUpRoute; 2] {
    let skills = &inventory.player_skills;
    [
        intrinsic_route(
            "railjack_intrinsics",
            "Rank up Railjack intrinsics",
            &[
                ("Tactical", skills.tactical),
                ("Piloting", skills.piloting),
                ("Gunnery", skills.gunnery),
                ("Engineering", skills.engineering),
                ("Command", skills.command),
            ],
        ),
        intrinsic_route(
            "duviri_intrinsics",
            "Rank up Drifter intrinsics",
            &[
                ("Combat", skills.drift_combat),
                ("Riding", skills.drift_riding),
                ("Opportunity", skills.drift_opportunity),
                ("Endurance", skills.drift_endurance),
            ],
        ),
    ]
}

pub(super) fn row(kind: &'static str, name: &str) -> MasteryItem {
    MasteryItem {
        unique_name: kind.to_owned(),
        name: name.to_owned(),
        kind,
        group: MasteryGroup::Other,
        image_name: None,
        owned: true,
        mastered: false,
        level: Level {
            current: 0,
            max: 0,
            xp_remaining: INTRINSIC_MASTERY_XP,
        },
        acquisition: Acquisition {
            missing_parts: INTRINSIC_RANKS_PER_SCHOOL as usize,
            plat_cost: 0,
            purchasable: false,
            relic_probability: 0.0,
        },
        favourite: false,
        components: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;

    #[test]
    fn fixture_intrinsics() {
        let inventory = fixtures::inventory();
        let (railjack, duviri) = intrinsics(&inventory);
        assert_eq!(railjack.current, 50);
        assert_eq!(railjack.max, 50);
        assert_eq!(duviri.current, 40);
        assert_eq!(duviri.max, 40);
    }
}
