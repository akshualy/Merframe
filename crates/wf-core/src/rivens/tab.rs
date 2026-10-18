use indexmap::IndexMap;
use wf_inventory::{Inventory, RivenChallenge, Upgrade};

use super::grading::compat_path;
use super::{Grader, RIVEN_MOD_SUFFIX, RivenRow, RivensTab, VeiledGroup, VeiledRiven};
use crate::catalog::display_name_from_path;
use crate::identity::ItemTable;
use crate::listings::MarketListings;

pub(super) fn weapon_class(name: &str) -> String {
    name.strip_suffix(RIVEN_MOD_SUFFIX)
        .unwrap_or(name)
        .to_owned()
}

impl VeiledRiven {
    fn new(
        riven_id: String,
        item_type: &str,
        count: i64,
        challenge: Option<&RivenChallenge>,
        items: &ItemTable,
    ) -> Self {
        let record = items.resolve(item_type);
        Self {
            riven_id,
            item_type: item_type.to_owned(),
            name: record.name.clone(),
            weapon_class: weapon_class(&record.name),
            market_slug: record.market_slug.clone(),
            image_name: record.image_name.clone(),
            count,
            progress: challenge.map_or(-1, |challenge| challenge.progress),
            required: challenge.map_or(-1, |challenge| challenge.required),
            pre_veiled: challenge.is_none(),
        }
    }
}

fn veiled(inventory: &Inventory, items: &ItemTable) -> Vec<VeiledGroup> {
    let mut groups: IndexMap<String, VeiledGroup> = IndexMap::new();
    let mut push = |challenge_id: String, complication: Option<String>, riven: VeiledRiven| {
        let group = groups
            .entry(challenge_id)
            .or_insert_with_key(|challenge_id| VeiledGroup {
                challenge_id: challenge_id.clone(),
                challenge: display_name_from_path(challenge_id),
                complication: complication.as_deref().map(display_name_from_path),
                count: 0,
                rivens: Vec::new(),
            });
        group.count += riven.count;
        group.rivens.push(riven);
    };
    for upgrade in inventory.rivens() {
        let Some(challenge) = upgrade
            .fingerprint()
            .and_then(|fingerprint| fingerprint.challenge)
        else {
            continue;
        };
        push(
            challenge.challenge_type.clone(),
            challenge.complication.clone(),
            VeiledRiven::new(
                upgrade.item_id.as_str().to_owned(),
                &upgrade.item_type,
                1,
                Some(&challenge),
                items,
            ),
        );
    }
    for stack in inventory.pre_veiled_rivens() {
        push(
            "Unrevealed".to_owned(),
            None,
            VeiledRiven::new(
                format!("{}#unrevealed", stack.item_type),
                &stack.item_type,
                stack.item_count,
                None,
                items,
            ),
        );
    }
    for group in groups.values_mut() {
        group.rivens.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.riven_id.cmp(&right.riven_id))
        });
    }
    groups.into_values().collect()
}

pub fn display_name(row: &RivenRow) -> Option<String> {
    Some(format!("{} {}", row.weapon, row.name.as_ref()?))
}

impl Grader<'_> {
    pub(crate) fn tab(&self, inventory: &Inventory, listings: &MarketListings) -> RivensTab {
        RivensTab {
            veiled: veiled(inventory, self.items),
            unveiled: self.rows(inventory, listings),
            attribution: self.attribution(),
        }
    }

    pub(crate) fn attribution(&self) -> Option<String> {
        self.table.map(|table| table.attribution.clone())
    }

    pub(super) fn rows(&self, inventory: &Inventory, listings: &MarketListings) -> Vec<RivenRow> {
        let mut rows: Vec<RivenRow> = inventory
            .rivens()
            .filter_map(|upgrade| self.row(upgrade, inventory, listings))
            .filter(|row| row.unveiled)
            .collect();
        rows.sort_by(|left, right| {
            right
                .grade
                .total_cmp(&left.grade)
                .then_with(|| left.item_id.cmp(&right.item_id))
        });
        rows
    }

    pub(crate) fn unveiled_row(
        &self,
        item_id: &str,
        inventory: &Inventory,
        listings: &MarketListings,
    ) -> Option<RivenRow> {
        let upgrade = inventory
            .rivens()
            .find(|upgrade| upgrade.item_id.as_str() == item_id)?;
        self.row(upgrade, inventory, listings)
            .filter(|row| row.unveiled)
    }

    fn row(
        &self,
        upgrade: &Upgrade,
        inventory: &Inventory,
        listings: &MarketListings,
    ) -> Option<RivenRow> {
        let catalog = self.catalog;
        let fingerprint = upgrade.fingerprint()?;
        let riven_type = self.riven_type(&upgrade.item_type);
        let weapon_path = fingerprint.compat.as_deref().map(compat_path);
        let weapon_item = weapon_path.and_then(|path| catalog.item(path));
        let (disposition, disposition_weapon) = match (weapon_item, weapon_path) {
            (Some(item), Some(path)) => {
                self.disposition_shown(path, &item.name, item.omega_attenuation, Some(inventory))
            }
            _ => (None, None),
        };
        let mod_item = catalog.item(&upgrade.item_type);
        let identity = RivenRow {
            item_id: upgrade.item_id.as_str().to_owned(),
            item_type: upgrade.item_type.clone(),
            weapon_class: riven_type.map(|found| weapon_class(&found.name)),
            riven_type: riven_type.map(|found| found.name.clone()),
            weapon: weapon_item.map_or_else(
                || self.items.name(weapon_path.unwrap_or(&upgrade.item_type)),
                |item| item.name.clone(),
            ),
            weapon_path: weapon_path.map(str::to_owned),
            weapon_slug: weapon_item
                .and_then(|item| self.items.get(&item.unique_name))
                .and_then(|record| record.market_slug.clone()),
            image_name: weapon_item
                .or(mod_item)
                .and_then(|item| item.image_name.clone()),
            disposition,
            disposition_weapon,
            name: None,
            unveiled: false,
            rank: 0,
            rank_required: None,
            rerolls: 0,
            polarity: None,
            grade: 0.0,
            grade_letter: "D",
            attributes: Vec::new(),
            good_roll: None,
            listed_in_wfm: false,
            unlisted_stat: None,
            pending: None,
        };
        let row = self.roll(identity, &fingerprint, riven_type);
        let listed_in_wfm = match (&row.name, &row.weapon_slug, row.rank_required) {
            (Some(name), Some(slug), Some(mastery)) => {
                listings.lists_riven(name, slug, mastery, row.rerolls)
            }
            _ => false,
        };
        Some(RivenRow {
            listed_in_wfm,
            ..row
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::rivens::grading::{perfectness, roll_grade};
    use crate::rivens::listing::{ListingChoices, listing_payload};
    use crate::rivens::support::*;
    use wf_market::OrderType;

    #[test]
    fn listed_riven_marked() {
        let inventory = fixtures::inventory();
        let fixture = fixture();
        let listed = riven_tab()
            .unveiled
            .into_iter()
            .find(|row| {
                row.name.is_some() && row.weapon_slug.is_some() && row.rank_required.is_some()
            })
            .expect("listable riven");
        let mut auctions = wf_market::parse_v1_auctions(fixtures::MY_AUCTIONS).unwrap();
        let auction = auctions.first_mut().unwrap();
        auction.item.name = listed.name.clone().unwrap();
        auction.item.weapon_url_name = listed.weapon_slug.clone().unwrap();
        auction.item.mastery_level = listed.rank_required.unwrap();
        auction.item.re_rolls = listed.rerolls;

        let marked = fixture.grader().rows(
            &inventory,
            &MarketListings::new(Vec::<(String, OrderType)>::new(), &auctions),
        );
        let mine = marked
            .iter()
            .find(|row| row.item_id == listed.item_id)
            .unwrap();
        assert!(mine.listed_in_wfm);
        assert_eq!(
            marked.iter().filter(|row| row.listed_in_wfm).count(),
            1,
            "no other riven is claimed by that auction"
        );
        assert!(
            riven_tab().unveiled.iter().all(|row| !row.listed_in_wfm),
            "without a market session no riven claims a listing"
        );
    }

    fn rerolled_inventory(item_id: &str) -> Inventory {
        fixtures::inventory_with(|value| {
            let entry = fixtures::entries(value, "Upgrades")
                .iter_mut()
                .find(|entry| entry["ItemId"]["$oid"] == item_id)
                .unwrap();
            let raw = entry["UpgradeFingerprint"].as_str().unwrap();
            let mut fingerprint: serde_json::Value = serde_json::from_str(raw).unwrap();
            let rerolls = fingerprint["rerolls"].as_u64().unwrap_or(0);
            fingerprint["rerolls"] = serde_json::json!(rerolls + 1);
            fingerprint["buffs"][0]["Value"] = serde_json::json!(BEST_ROLL);
            entry["UpgradeFingerprint"] =
                serde_json::json!(serde_json::to_string(&fingerprint).unwrap());
        })
    }

    #[test]
    fn letter_of_the_mean_grade() {
        let tab = riven_tab();

        assert!(!tab.unveiled.is_empty());
        assert!(
            tab.unveiled
                .iter()
                .all(|row| row.grade_letter == roll_grade(perfectness(&row.attributes)))
        );
    }

    #[test]
    fn display_names() {
        let tab = riven_tab();
        let row = by_weapon(&tab.unveiled, "Soma");
        assert_eq!(display_name(row).as_deref(), Some("Soma Toxidex"));
        assert_eq!(
            display_name(by_weapon(&tab.unveiled, "Nepheri")).as_deref(),
            Some("Nepheri Geli-toxidra")
        );
    }

    #[test]
    fn unveiled_rows() {
        let tab = riven_tab();
        assert_eq!(tab.unveiled.len(), 13);
        assert!(tab.unveiled.iter().all(|row| row.unveiled));
        assert!(tab.unveiled.iter().all(|row| !row.attributes.is_empty()));
        assert!(tab.unveiled.iter().all(|row| {
            row.weapon_class
                .as_deref()
                .is_some_and(|class| !class.is_empty() && !class.ends_with("Riven Mod"))
        }));
    }

    #[test]
    fn veiled_groups_by_challenge() {
        let tab = riven_tab();
        assert_eq!(tab.veiled.len(), 3);
        assert_eq!(tab.veiled.iter().map(|group| group.count).sum::<i64>(), 184);
        assert!(tab.veiled.iter().all(|group| !group.rivens.is_empty()));
        assert!(
            tab.veiled
                .iter()
                .all(|group| group.count
                    == group.rivens.iter().map(|riven| riven.count).sum::<i64>())
        );

        let kills = tab
            .veiled
            .iter()
            .find(|group| group.challenge_id == "/Lotus/Types/Challenges/RandomizedKill")
            .expect("kill group");
        assert_eq!(kills.challenge, "Randomized Kill");
        assert_eq!(kills.complication.as_deref(), Some("Reset On New Day"));
        assert_eq!(kills.rivens.len(), 1);
        assert_eq!(kills.rivens[0].count, 1);
        assert_eq!(kills.rivens[0].required, 128);
        assert_eq!(kills.rivens[0].progress, 0);
        assert!(!kills.rivens[0].pre_veiled);
        assert_eq!(kills.rivens[0].weapon_class, "Kitgun");
        assert_eq!(kills.rivens[0].name, "Kitgun Riven Mod");
        assert_eq!(
            kills.rivens[0].market_slug.as_deref(),
            Some("kitgun_riven_mod_(veiled)")
        );

        let unrevealed = tab
            .veiled
            .iter()
            .find(|group| group.challenge_id == "Unrevealed")
            .expect("unrevealed group");
        assert_eq!(unrevealed.count, 182);
        assert_eq!(unrevealed.rivens.len(), 8);
        assert!(unrevealed.rivens.iter().all(|riven| riven.pre_veiled));
        assert!(unrevealed.complication.is_none());
        assert!(
            unrevealed
                .rivens
                .iter()
                .all(|riven| riven.progress == -1 && riven.required == -1)
        );
        assert!(
            unrevealed
                .rivens
                .iter()
                .all(|riven| riven.riven_id.ends_with("#unrevealed"))
        );
        let classes: Vec<&str> = unrevealed
            .rivens
            .iter()
            .map(|riven| riven.weapon_class.as_str())
            .collect();
        assert_eq!(
            classes,
            [
                "Archgun",
                "Companion Weapon",
                "Kitgun",
                "Melee",
                "Pistol",
                "Rifle",
                "Shotgun",
                "Zaw",
            ]
        );
        assert!(
            unrevealed
                .rivens
                .iter()
                .all(|riven| riven.name.ends_with(RIVEN_MOD_SUFFIX))
        );
    }

    #[test]
    fn every_unveiled_riven_resolves() {
        let tab = riven_tab();
        let mut unmapped: Vec<String> = Vec::new();
        for row in &tab.unveiled {
            assert!(
                row.name.as_deref().is_some_and(|name| !name.is_empty()),
                "{} has no generated name",
                row.item_id
            );
            assert!(row.weapon_slug.is_some(), "{} has no slug", row.item_id);
            assert!(
                row.riven_type.is_some(),
                "{} has no riven type",
                row.item_id
            );
            assert!(
                row.disposition.is_some(),
                "{} has no disposition",
                row.item_id
            );
            assert!(
                row.rank_required.is_some(),
                "{} has no mastery requirement",
                row.item_id
            );
            for attribute in &row.attributes {
                if attribute.slug.is_none()
                    || attribute.rolled.is_none()
                    || attribute.min.is_none()
                    || attribute.max.is_none()
                {
                    unmapped.push(format!("{} {}", row.item_id, attribute.tag));
                }
            }
            let direct = ListingChoices {
                direct: true,
                selling_price: 100,
                ..ListingChoices::default()
            };
            assert!(
                listing_payload(row, &direct).is_some(),
                "{} cannot be listed",
                row.item_id
            );
        }
        assert!(unmapped.is_empty(), "unmapped attributes: {unmapped:?}");
    }

    #[test]
    fn rerolled_fixture_riven() {
        let tab = riven_tab();
        let before = by_weapon(&tab.unveiled, "Soma").clone();
        let after_rows = fixture().grader().rows(
            &rerolled_inventory(&before.item_id),
            &MarketListings::default(),
        );
        let after = after_rows
            .iter()
            .find(|row| row.item_id == before.item_id)
            .unwrap();

        assert_eq!(after.rerolls, before.rerolls + 1);
        assert_eq!(after.item_type, before.item_type);
        assert_eq!(after.weapon, before.weapon);
        assert_ne!(before.attributes[0].value, BEST_ROLL);
        assert_eq!(after.attributes[0].value, BEST_ROLL);
        assert_eq!(after.attributes[0].grade, "S");
        assert!(after.grade > before.grade);
    }

    #[test]
    fn good_rolls_and_attribution() {
        let with_table = riven_tab();
        assert!(
            with_table
                .attribution
                .as_deref()
                .is_some_and(|t| t.contains("44bananas"))
        );
        let nepheri = by_weapon(&with_table.unveiled, "Nepheri");
        let view = nepheri.good_roll.as_ref().unwrap();
        assert_eq!(view.alternatives.len(), 1);
        assert!(view.alternatives[0].mandatory.is_empty());
        assert_eq!(view.alternatives[0].optional_needed, 3);
        assert!(!view.alternatives[0].complete);
        assert!(!view.matches);
        assert!(
            with_table
                .unveiled
                .iter()
                .filter(|row| row.good_roll.is_some())
                .count()
                > 8
        );

        let fixture = fixture();
        let without = Grader::new(&fixture.catalog, &fixture.items, None)
            .tab(&fixtures::inventory(), &MarketListings::default());
        assert!(without.attribution.is_none());
        assert!(without.unveiled.iter().all(|row| row.good_roll.is_none()));
    }
}
