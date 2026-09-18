use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::item::Item;

pub const COMBO_POINTS_TAG: &str = "WeaponMeleeComboPointsOnHitMod";
const ROLL_MIN: f64 = 0.9;
const ROLL_MAX: f64 = 1.1;
pub const MAX_RANK: u32 = 8;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpgradeValue {
    pub value: f64,
    #[serde(rename = "locTag")]
    pub loc_tag: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpgradeEntry {
    pub tag: String,
    #[serde(rename = "prefixTag")]
    pub prefix_tag: String,
    #[serde(rename = "suffixTag")]
    pub suffix_tag: String,
    #[serde(rename = "upgradeValues")]
    pub upgrade_values: Vec<UpgradeValue>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraitMultipliers {
    pub good: f64,
    pub bad: f64,
}

pub fn trait_multipliers(buffs: usize, curses: usize) -> Option<TraitMultipliers> {
    let (good, bad) = match (buffs, curses) {
        (2, 0) => (0.99, 0.0),
        (2, 1) => (1.2375, -0.495),
        (3, 0) => (0.75, 0.0),
        (3, 1) => (0.9375, -0.75),
        _ => return None,
    };
    Some(TraitMultipliers { good, bad })
}

#[derive(Debug, Clone, PartialEq)]
pub struct RivenStat {
    pub tag: String,
    pub prefix: String,
    pub suffix: String,
    pub base_value: f64,
    pub localization: String,
}

impl RivenStat {
    pub fn percent(&self) -> bool {
        self.localization.contains("|val|%") || self.localization.contains("|STAT1|%")
    }

    pub fn multiplier_display(&self) -> bool {
        self.localization.contains("Damage to")
    }

    pub fn prefix_suffix(&self) -> String {
        format!(
            "{}|{}",
            self.prefix.to_lowercase(),
            self.suffix.to_lowercase()
        )
    }

    pub fn base_at_rank_nine(&self, disposition: f64, multiplier: f64, curse: bool) -> f64 {
        let mut base = 90.0 * self.base_value * disposition * multiplier;
        if curse && self.tag == COMBO_POINTS_TAG && base > 0.0 {
            base = -base;
        }
        if self.percent() {
            base *= 100.0;
        }
        base
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RivenType {
    pub unique_name: String,
    pub name: String,
    pub stats: HashMap<String, RivenStat>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RivenData {
    types: HashMap<String, RivenType>,
}

impl RivenData {
    pub fn from_items(items: &[Item]) -> Self {
        let types = items
            .iter()
            .filter_map(|item| {
                let entries = item.upgrade_entries.as_deref()?;
                let stats = entries
                    .iter()
                    .filter_map(|entry| {
                        let first = entry.upgrade_values.first()?;
                        Some((
                            entry.tag.clone(),
                            RivenStat {
                                tag: entry.tag.clone(),
                                prefix: entry.prefix_tag.clone(),
                                suffix: entry.suffix_tag.clone(),
                                base_value: first.value,
                                localization: first.loc_tag.clone().unwrap_or_default(),
                            },
                        ))
                    })
                    .collect();
                Some((
                    item.unique_name.clone(),
                    RivenType {
                        unique_name: item.unique_name.clone(),
                        name: item.name.clone(),
                        stats,
                    },
                ))
            })
            .collect();
        Self { types }
    }

    pub fn riven_type(&self, unique_name: &str) -> Option<&RivenType> {
        self.types.get(unique_name)
    }

    pub fn stat(&self, riven_type: &str, tag: &str) -> Option<&RivenStat> {
        self.types.get(riven_type)?.stats.get(tag)
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "roll values are 30 bit integers"
)]
pub fn roll_multiplier(value: i64) -> f64 {
    (ROLL_MIN + value as f64 / 53_687_091.0 / 100.0).clamp(ROLL_MIN, ROLL_MAX)
}

pub fn roll_share(multiplier: f64, curse: bool) -> f64 {
    let share = (multiplier - ROLL_MIN) * 5.0;
    if curse { 1.0 - share } else { share }
}

pub fn rank_multiplier(rank: u32) -> f64 {
    f64::from(rank + 1) / 9.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEMS: &str = include_str!("../tests/fixtures/riven_items.json");
    const RIFLE: &str = "/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare";
    const MELEE: &str = "/Lotus/Upgrades/Mods/Randomized/PlayerMeleeWeaponRandomModRare";

    fn data() -> RivenData {
        let items: Vec<Item> = serde_json::from_str(ITEMS).unwrap();
        RivenData::from_items(&items)
    }

    #[test]
    fn riven_types_from_items() {
        let data = data();
        assert_eq!(data.len(), 7);
        assert!(!data.is_empty());
        let rifle = data.riven_type(RIFLE).unwrap();
        assert_eq!(rifle.name, "Rifle Riven Mod");
        assert_eq!(rifle.stats.len(), 24);
        assert!(data.riven_type("/Lotus/Nope").is_none());
    }

    #[test]
    fn stat_fields() {
        let data = data();
        let crit = data.stat(RIFLE, "WeaponCritChanceMod").unwrap();
        assert_eq!(crit.prefix, "crita");
        assert_eq!(crit.suffix, "cron");
        assert_eq!(crit.prefix_suffix(), "crita|cron");
        assert!((crit.base_value - 0.016_666).abs() < 1e-6);
        assert_eq!(crit.localization, "|val|% Critical Chance");
        assert!(crit.percent());
        assert!(!crit.multiplier_display());

        let corpus = data.stat(RIFLE, "WeaponFactionDamageCorpus").unwrap();
        assert!(!corpus.percent());
        assert!(corpus.multiplier_display());

        let combo = data.stat(MELEE, COMBO_POINTS_TAG).unwrap();
        assert_eq!(combo.prefix, "");
        assert_eq!(combo.suffix, "");
        assert!(combo.base_value < 0.0);
    }

    #[test]
    fn trait_multiplier_table() {
        assert_eq!(
            trait_multipliers(2, 0),
            Some(TraitMultipliers {
                good: 0.99,
                bad: 0.0
            })
        );
        assert_eq!(
            trait_multipliers(2, 1),
            Some(TraitMultipliers {
                good: 1.2375,
                bad: -0.495
            })
        );
        assert_eq!(
            trait_multipliers(3, 0),
            Some(TraitMultipliers {
                good: 0.75,
                bad: 0.0
            })
        );
        assert_eq!(
            trait_multipliers(3, 1),
            Some(TraitMultipliers {
                good: 0.9375,
                bad: -0.75
            })
        );
        assert_eq!(trait_multipliers(1, 0), None);
        assert_eq!(trait_multipliers(4, 1), None);
        assert_eq!(trait_multipliers(3, 2), None);
    }

    #[test]
    fn roll_multiplier_range() {
        assert!((roll_multiplier(0) - 0.9).abs() < f64::EPSILON);
        assert!((roll_multiplier(-1) - 0.9).abs() < f64::EPSILON);
        assert!((roll_multiplier(1 << 30) - 1.1).abs() < 1e-9);
        assert!((roll_multiplier(1 << 40) - 1.1).abs() < f64::EPSILON);
        let half = roll_multiplier(536_870_912);
        assert!((half - 1.0).abs() < 1e-6, "{half}");
        assert!((roll_share(1.1, false) - 1.0).abs() < 1e-9);
        assert!((roll_share(1.1, true) - 0.0).abs() < 1e-9);
        assert!((roll_share(0.9, true) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rank_multiplier_at_max() {
        assert!((rank_multiplier(MAX_RANK) - 1.0).abs() < f64::EPSILON);
        assert!((rank_multiplier(0) - 1.0 / 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rifle_damage_published_range() {
        let data = data();
        let damage = data.stat(RIFLE, "WeaponDamageAmountMod").unwrap();
        let multipliers = trait_multipliers(2, 1).unwrap();
        let base = damage.base_at_rank_nine(1.0, multipliers.good, false);
        assert!((base * 0.9 - 183.77).abs() < 0.01, "{}", base * 0.9);
        assert!((base * 1.1 - 224.61).abs() < 0.01, "{}", base * 1.1);

        let recoil = data.stat(RIFLE, "WeaponRecoilReductionMod").unwrap();
        assert!(recoil.base_value < 0.0);
        assert!(recoil.base_at_rank_nine(1.0, multipliers.good, false) < 0.0);
    }

    #[test]
    fn combo_points_curse() {
        let data = data();
        let combo = data.stat(MELEE, COMBO_POINTS_TAG).unwrap();
        let multipliers = trait_multipliers(2, 1).unwrap();
        let as_curse = combo.base_at_rank_nine(1.0, multipliers.bad, true);
        assert!(as_curse < 0.0, "{as_curse}");
        let unfixed = combo.base_at_rank_nine(1.0, multipliers.bad, false);
        assert!(unfixed > 0.0, "{unfixed}");
    }
}
