use std::collections::HashSet;

use wf_data::{DEFAULT_MAX_RANK, mastery_level_from_affinity};
use wf_inventory::Inventory;

use super::intrinsics::INTRINSIC_MASTERY_XP;
use super::items::masterable;
use super::star_chart::star_chart_xp;
use crate::account::Account;
use crate::catalog::Catalog;

const WARFRAME_AFFINITY_CAP: u64 = 900_000;
pub(super) const WARFRAME_MASTERY_PER_RANK: u64 = 200;

pub(super) const VENARI: [(&str, &str); 2] = [
    (
        "/Lotus/Powersuits/Khora/Kavat/KhoraKavatPowerSuit",
        "Venari",
    ),
    (
        "/Lotus/Powersuits/Khora/Kavat/KhoraPrimeKavatPowerSuit",
        "Venari Prime",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MasteryXp {
    pub items: u64,
    pub plexus: u64,
    pub venari: u64,
    pub unlisted: u64,
    pub star_chart: u64,
    pub intrinsics: u64,
}

impl MasteryXp {
    pub fn total(self) -> u64 {
        self.items + self.plexus + self.venari + self.unlisted + self.star_chart + self.intrinsics
    }
}

pub(super) fn warframe_rank(affinity: u64) -> u64 {
    mastery_level_from_affinity(affinity, true, WARFRAME_AFFINITY_CAP)
}

pub(super) fn xp_for_rank(rank: u32) -> u64 {
    let rank = u64::from(rank);
    if rank <= 30 {
        2500 * rank * rank
    } else {
        2_250_000 + 147_500 * (rank - 30)
    }
}

pub(super) fn counted_affinity_types<'a>(
    account: &'a Account,
    catalog: &'a Catalog,
) -> HashSet<&'a str> {
    let mut counted: HashSet<&str> = masterable(catalog, true)
        .filter(|item| account.affinity_of(&item.unique_name) > 0)
        .map(|item| item.unique_name.as_str())
        .collect();
    counted.extend(
        account
            .inventory
            .crew_ship_harnesses
            .iter()
            .map(|harness| harness.item_type.as_str()),
    );
    counted.extend(VENARI.map(|(unique_name, _)| unique_name));
    counted
}

fn ranks_like_a_warframe(item_type: &str) -> bool {
    item_type.contains("/Powersuits/") || item_type.contains("/Sentinel/")
}

pub(super) fn unlisted_ranks<'a>(
    inventory: &'a Inventory,
    counted: &HashSet<&'a str>,
) -> Vec<(&'a str, u64, u64)> {
    let weapon_cap = 500 * u64::from(DEFAULT_MAX_RANK).pow(2);
    inventory
        .xp_info
        .iter()
        .filter(|entry| entry.xp > 0)
        .filter(|entry| !counted.contains(entry.item_type.as_str()))
        .map(|entry| {
            let item_type = entry.item_type.as_str();
            if ranks_like_a_warframe(item_type) {
                (
                    item_type,
                    WARFRAME_MASTERY_PER_RANK,
                    warframe_rank(entry.xp),
                )
            } else {
                (
                    item_type,
                    100,
                    mastery_level_from_affinity(entry.xp, false, weapon_cap),
                )
            }
        })
        .collect()
}

pub(super) fn mastery_xp(account: &Account, catalog: &Catalog) -> MasteryXp {
    let inventory = &account.inventory;
    let items_xp: u64 = masterable(catalog, true)
        .map(|item| {
            let affinity = account.affinity_of(&item.unique_name);
            u64::from(item.mastery_per_rank() * item.mastery_rank_at(affinity))
        })
        .sum();

    let plexus: u64 = inventory
        .crew_ship_harnesses
        .iter()
        .map(|harness| warframe_rank(harness.xp) * WARFRAME_MASTERY_PER_RANK)
        .sum();

    let venari: u64 = VENARI
        .into_iter()
        .map(|(unique_name, _)| {
            warframe_rank(account.affinity_of(unique_name)) * WARFRAME_MASTERY_PER_RANK
        })
        .sum();

    let counted = counted_affinity_types(account, catalog);
    let unlisted: u64 = unlisted_ranks(inventory, &counted)
        .into_iter()
        .map(|(_, per_rank, rank)| per_rank * rank)
        .sum();

    let skills = &inventory.player_skills;
    MasteryXp {
        items: items_xp,
        plexus,
        venari,
        unlisted,
        star_chart: star_chart_xp(inventory),
        intrinsics: INTRINSIC_MASTERY_XP * u64::from(skills.railjack() + skills.duviri()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;

    #[test]
    fn rank_thresholds() {
        assert_eq!(xp_for_rank(0), 0);
        assert_eq!(xp_for_rank(1), 2500);
        assert_eq!(xp_for_rank(30), 2_250_000);
        assert_eq!(xp_for_rank(31), 2_397_500);
        assert_eq!(xp_for_rank(36), 3_135_000);
        assert_eq!(xp_for_rank(37), 3_282_500);
    }

    #[test]
    fn fixture_xp_breakdown() {
        let account = Account::new(fixtures::inventory());
        let inventory = &account.inventory;
        let catalog = fixtures::mastery_catalog();
        let breakdown = mastery_xp(&account, &catalog);

        assert_eq!(breakdown.items, 324_000);
        assert_eq!(breakdown.plexus, 6000);
        assert_eq!(breakdown.venari, 12_000);
        assert_eq!(breakdown.unlisted, 3000);
        assert_eq!(breakdown.star_chart, 55_138);
        assert_eq!(breakdown.intrinsics, 135_000);
        assert_eq!(breakdown.total(), 535_138);

        let with_skins = mastery_xp(&account, &fixtures::with_skins(fixtures::MASTERY_ITEMS));
        assert_eq!(with_skins.items, breakdown.items);
        assert_eq!(with_skins.total(), breakdown.total());

        assert!(
            breakdown.total() >= xp_for_rank(inventory.player_level),
            "an MR{} account must have earned at least {} XP, got {}",
            inventory.player_level,
            xp_for_rank(inventory.player_level),
            breakdown.total()
        );
        assert!(breakdown.total() < xp_for_rank(inventory.player_level + 1));
    }
}
