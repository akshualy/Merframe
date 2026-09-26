use std::collections::HashMap;

use serde::Serialize;
use wf_market::{Polarity, RivenAttribute, RivenData as RivenTable};

use crate::catalog::Catalog;

mod dialog;
mod good_roll;
mod grading;
mod listing;
mod reroll;
mod station;
mod tab;

pub use good_roll::{AlternativeMatch, GoodRollView, StatMatch};
pub use grading::{AttributeGrade, fingerprint_weapon};
pub use listing::{ListingChoices, listing_payload};
pub use reroll::{KeptRoll, kept_roll};
pub use tab::display_name;

const RIVEN_MOD_SUFFIX: &str = " Riven Mod";

pub(crate) struct Grader<'a> {
    catalog: &'a Catalog,
    by_combo: HashMap<String, &'a RivenAttribute>,
    table: Option<&'a RivenTable>,
}

impl<'a> Grader<'a> {
    pub(crate) fn new(
        catalog: &'a Catalog,
        attributes: &'a [RivenAttribute],
        table: Option<&'a RivenTable>,
    ) -> Self {
        let by_combo = attributes
            .iter()
            .map(|attribute| {
                (
                    format!(
                        "{}|{}",
                        attribute.prefix.to_lowercase(),
                        attribute.suffix.to_lowercase()
                    ),
                    attribute,
                )
            })
            .collect();
        Self {
            catalog,
            by_combo,
            table,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PendingRoll {
    pub name: Option<String>,
    pub rerolls: u32,
    pub polarity: Option<Polarity>,
    pub grade: f64,
    pub attributes: Vec<AttributeGrade>,
    pub good_roll: Option<GoodRollView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RivenRow {
    pub item_id: String,
    pub item_type: String,
    pub riven_type: Option<String>,
    pub weapon_class: Option<String>,
    pub name: Option<String>,
    pub weapon: Option<String>,
    pub weapon_path: Option<String>,
    pub weapon_slug: Option<String>,
    pub image_name: Option<String>,
    pub disposition: Option<f64>,
    pub disposition_weapon: Option<String>,
    pub unveiled: bool,
    pub rank: u32,
    pub rank_required: Option<u32>,
    pub rerolls: u32,
    pub polarity: Option<Polarity>,
    pub grade: f64,
    pub attributes: Vec<AttributeGrade>,
    pub good_roll: Option<GoodRollView>,
    pub listed_in_wfm: bool,
    pub pending: Option<PendingRoll>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VeiledRiven {
    pub riven_id: String,
    pub item_type: String,
    pub name: Option<String>,
    pub weapon_class: Option<String>,
    pub image_name: Option<String>,
    pub count: i64,
    pub progress: i64,
    pub required: i64,
    pub pre_veiled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VeiledGroup {
    pub challenge_id: String,
    pub challenge: String,
    pub complication: Option<String>,
    pub count: i64,
    pub rivens: Vec<VeiledRiven>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RivensTab {
    pub veiled: Vec<VeiledGroup>,
    pub unveiled: Vec<RivenRow>,
    pub attribution: Option<String>,
}

#[cfg(test)]
pub(crate) mod support {
    use super::*;
    use crate::catalog::{Catalog, fixtures};
    use crate::listings::MarketListings;
    use wf_market::{RivenAttribute, RivenData as RivenTable, RivenWeapon};

    pub(crate) const ATTRIBUTES: &str =
        include_str!("../../../wf-market/tests/fixtures/riven_attributes.json");

    pub(crate) const RIVEN_DATA: &str = include_str!("../../../../fixtures/riven_data.json");

    pub(crate) const CURRENT_ROLL: &str = r#"{"compat":"/Lotus/Weapons/Archon/Melee/DualDaggers/ArchonDualDaggersPlayerWep","lim":294146365,"lvlReq":11,"lvl":8,"rerolls":74,"pol":"AP_DEFENSE","buffs":[{"Tag":"WeaponFreezeDamageMod","Value":708669601},{"Tag":"WeaponToxinDamageMod","Value":526133492},{"Tag":"WeaponFireRateMod","Value":397284473}],"curses":[]}"#;

    pub(crate) const PENDING_ROLL: &str = r#"{"compat":"/Lotus/Weapons/Archon/Melee/DualDaggers/ArchonDualDaggersPlayerWep","lim":294146365,"lvlReq":11,"lvl":8,"rerolls":75,"pol":"AP_DEFENSE","buffs":[{"Tag":"WeaponFreezeDamageMod","Value":741320000},{"Tag":"WeaponCritDamageMod","Value":903450000},{"Tag":"WeaponFireDamageMod","Value":512780000}],"curses":[{"Tag":"SlideAttackCritChanceMod","Value":268940000}]}"#;

    pub(crate) const PRE_VEILED_ITEMS: &str = r#"[
      {
            "uniqueName": "/Lotus/Upgrades/Mods/Randomized/RawArchgunRandomMod",
            "name": "Archgun Riven Mod",
            "category": "Mods",
            "type": "Archgun Riven Mod",
            "tradable": false,
            "imageName": "OmegaMod.png"
      },
      {
            "uniqueName": "/Lotus/Upgrades/Mods/Randomized/RawSentinelWeaponRandomMod",
            "name": "Companion Weapon Riven Mod",
            "category": "Mods",
            "type": "Companion Weapon Riven Mod",
            "tradable": false,
            "imageName": "OmegaMod.png"
      },
      {
            "uniqueName": "/Lotus/Upgrades/Mods/Randomized/RawModularPistolRandomMod",
            "name": "Kitgun Riven Mod",
            "category": "Mods",
            "type": "Kitgun Riven Mod",
            "tradable": false,
            "imageName": "OmegaMod.png"
      },
      {
            "uniqueName": "/Lotus/Upgrades/Mods/Randomized/RawMeleeRandomMod",
            "name": "Melee Riven Mod",
            "category": "Mods",
            "type": "Melee Riven Mod",
            "tradable": false,
            "imageName": "OmegaMod.png"
      },
      {
            "uniqueName": "/Lotus/Upgrades/Mods/Randomized/RawPistolRandomMod",
            "name": "Pistol Riven Mod",
            "category": "Mods",
            "type": "Pistol Riven Mod",
            "tradable": false,
            "imageName": "OmegaMod.png"
      },
      {
            "uniqueName": "/Lotus/Upgrades/Mods/Randomized/RawRifleRandomMod",
            "name": "Rifle Riven Mod",
            "category": "Mods",
            "type": "Rifle Riven Mod",
            "tradable": false,
            "imageName": "OmegaMod.png"
      },
      {
            "uniqueName": "/Lotus/Upgrades/Mods/Randomized/RawShotgunRandomMod",
            "name": "Shotgun Riven Mod",
            "category": "Mods",
            "type": "Shotgun Riven Mod",
            "tradable": false,
            "imageName": "OmegaMod.png"
      },
      {
            "uniqueName": "/Lotus/Upgrades/Mods/Randomized/RawModularMeleeRandomMod",
            "name": "Zaw Riven Mod",
            "category": "Mods",
            "type": "Zaw Riven Mod",
            "tradable": false,
            "imageName": "OmegaMod.png"
      }
    ]"#;

    pub(crate) const RIFLE_RIVEN: &str = "/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare";

    pub(crate) const MIDDLE_ROLL: i64 = 536_870_910;

    pub(crate) const BEST_ROLL: i64 = 1_073_741_820;

    pub(crate) fn attributes() -> Vec<RivenAttribute> {
        wf_market::envelope::<Vec<RivenAttribute>>(ATTRIBUTES).unwrap()
    }

    pub(crate) fn riven_table() -> RivenTable {
        wf_market::parse_riven_data(RIVEN_DATA).expect("riven data")
    }

    pub(crate) fn table_weapon_named(table: &RivenTable, name: &str) -> RivenWeapon {
        table
            .weapons
            .iter()
            .find(|weapon| weapon.name == name)
            .unwrap()
            .clone()
    }

    pub(crate) fn rolled(compat: &str, buffs: &[&str], curse: Option<&str>) -> String {
        let stats = |tags: &[&str]| {
            tags.iter()
                .map(|tag| format!(r#"{{"Tag":"{tag}","Value":{MIDDLE_ROLL}}}"#))
                .collect::<Vec<String>>()
                .join(",")
        };
        let curses = curse.map(|tag| stats(&[tag])).unwrap_or_default();
        format!(
            r#"{{"compat":"{compat}","lvlReq":16,"lvl":8,"rerolls":1,"pol":"AP_DEFENSE","buffs":[{}],"curses":[{curses}]}}"#,
            stats(buffs)
        )
    }

    pub(crate) fn riven_catalog() -> Catalog {
        let mut items: Vec<serde_json::Value> =
            serde_json::from_str(fixtures::RIVEN_ITEMS).unwrap();
        let stacks: Vec<serde_json::Value> = serde_json::from_str(PRE_VEILED_ITEMS).unwrap();
        items.extend(stacks);
        let json = serde_json::to_string(&items).unwrap();
        Catalog::from_json(&json, fixtures::RELICS, fixtures::COMPONENTS).unwrap()
    }

    pub(crate) struct Fixture {
        pub(crate) catalog: Catalog,
        pub(crate) attributes: Vec<RivenAttribute>,
        pub(crate) table: RivenTable,
    }

    impl Fixture {
        pub(crate) fn grader(&self) -> Grader<'_> {
            Grader::new(&self.catalog, &self.attributes, Some(&self.table))
        }
    }

    pub(crate) fn fixture() -> Fixture {
        Fixture {
            catalog: riven_catalog(),
            attributes: attributes(),
            table: riven_table(),
        }
    }

    pub(crate) fn riven_tab() -> RivensTab {
        fixture()
            .grader()
            .tab(&fixtures::inventory(), &MarketListings::default())
    }

    pub(crate) fn by_weapon<'a>(rows: &'a [RivenRow], weapon: &str) -> &'a RivenRow {
        rows.iter()
            .find(|row| row.weapon.as_deref() == Some(weapon))
            .unwrap()
    }
}
