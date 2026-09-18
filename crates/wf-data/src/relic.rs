use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{DataError, Result};
use crate::item::{MarketSlug, Rarity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Refinement {
    Intact,
    Exceptional,
    Flawless,
    Radiant,
}

const REFINEMENT_SUFFIXES: [(&str, Refinement); 4] = [
    (" Intact", Refinement::Intact),
    (" Exceptional", Refinement::Exceptional),
    (" Flawless", Refinement::Flawless),
    (" Radiant", Refinement::Radiant),
];

const PROJECTION_SUFFIXES: [(&str, Refinement); 4] = [
    ("Bronze", Refinement::Intact),
    ("Silver", Refinement::Exceptional),
    ("Gold", Refinement::Flawless),
    ("Platinum", Refinement::Radiant),
];

fn split_refinement(name: &str) -> Option<(&str, Refinement)> {
    REFINEMENT_SUFFIXES
        .iter()
        .find_map(|(suffix, refinement)| name.strip_suffix(suffix).map(|base| (base, *refinement)))
}

fn relic_name(name: &str) -> Option<&str> {
    if let Some((base, _)) = split_refinement(name) {
        return Some(base);
    }
    let base = name.strip_suffix(" Relic")?;
    base.contains(' ').then_some(base)
}

fn projection_family(unique_name: &str) -> &str {
    PROJECTION_SUFFIXES
        .iter()
        .find_map(|(suffix, _)| unique_name.strip_suffix(suffix))
        .unwrap_or(unique_name)
}

fn projection_refinement(unique_name: &str) -> Refinement {
    PROJECTION_SUFFIXES
        .iter()
        .find(|(suffix, _)| unique_name.ends_with(suffix))
        .map_or(Refinement::Intact, |(_, refinement)| *refinement)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicReward {
    pub item_name: String,
    pub item_unique_name: String,
    pub rarity: Rarity,
    pub chance: f64,
    pub market_slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicDrop {
    pub location: String,
    pub chance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relic {
    pub tier: String,
    pub name: String,
    pub vaulted: bool,
    pub tradable: bool,
    pub market_info: Option<MarketSlug>,
    pub image_name: Option<String>,
    pub image_names: HashMap<Refinement, String>,
    pub unique_names: HashMap<Refinement, String>,
    pub rewards: HashMap<Refinement, Vec<RelicReward>>,
    pub drops: Vec<RelicDrop>,
}

impl Relic {
    pub fn rewards_for(&self, refinement: Refinement) -> &[RelicReward] {
        self.rewards.get(&refinement).map_or(&[], Vec::as_slice)
    }
}

#[derive(Debug, Deserialize)]
struct RawRelic {
    #[serde(rename = "uniqueName")]
    unique_name: String,
    name: String,
    #[serde(default)]
    vaulted: bool,
    tradable: bool,
    #[serde(default)]
    rewards: Vec<RawReward>,
    #[serde(rename = "marketInfo")]
    market_info: Option<MarketSlug>,
    #[serde(rename = "imageName")]
    image_name: Option<String>,
    #[serde(default)]
    drops: Vec<RawDrop>,
}

#[derive(Debug, Deserialize)]
struct RawDrop {
    location: String,
    chance: f64,
}

#[derive(Debug, Deserialize)]
struct RawReward {
    chance: f64,
    rarity: Rarity,
    item: RawRewardItem,
}

#[derive(Debug, Deserialize)]
struct RawRewardItem {
    name: String,
    #[serde(rename = "uniqueName")]
    unique_name: String,
    #[serde(rename = "warframeMarket")]
    warframe_market: Option<MarketSlug>,
}

fn best_chance_per_location(drops: Vec<RawDrop>) -> Vec<RelicDrop> {
    let mut best: HashMap<String, f64> = HashMap::new();
    for drop in drops {
        let slot = best.entry(drop.location).or_insert(drop.chance);
        if drop.chance > *slot {
            *slot = drop.chance;
        }
    }
    let mut ranked: Vec<RelicDrop> = best
        .into_iter()
        .map(|(location, chance)| RelicDrop { location, chance })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .chance
            .total_cmp(&left.chance)
            .then_with(|| left.location.cmp(&right.location))
    });
    ranked
}

pub(crate) fn parse_relics(json: &str) -> Result<Vec<Relic>> {
    let raw: Vec<RawRelic> =
        serde_json::from_str(json).map_err(|source| DataError::Parse("relic", source))?;
    let mut grouped: HashMap<String, Relic> = HashMap::new();
    for entry in raw {
        let Some(name) = relic_name(&entry.name) else {
            continue;
        };
        let refinement = projection_refinement(&entry.unique_name);
        let relic = grouped
            .entry(projection_family(&entry.unique_name).to_owned())
            .or_insert_with(|| Relic {
                tier: name
                    .split_once(' ')
                    .map_or(name, |(tier, _)| tier)
                    .to_owned(),
                name: name.to_owned(),
                vaulted: entry.vaulted,
                tradable: entry.tradable,
                market_info: entry.market_info.clone(),
                image_name: entry.image_name.clone(),
                image_names: HashMap::new(),
                unique_names: HashMap::new(),
                rewards: HashMap::new(),
                drops: Vec::new(),
            });
        if let Some(image) = entry.image_name.clone() {
            relic.image_names.insert(refinement, image);
        }
        if relic.drops.is_empty() {
            relic.drops = best_chance_per_location(entry.drops);
        }
        relic.unique_names.insert(refinement, entry.unique_name);
        relic.rewards.insert(
            refinement,
            entry
                .rewards
                .into_iter()
                .map(|reward| RelicReward {
                    item_name: reward.item.name,
                    item_unique_name: reward.item.unique_name,
                    rarity: reward.rarity,
                    chance: reward.chance,
                    market_slug: reward.item.warframe_market.map(|slug| slug.url_name),
                })
                .collect(),
        );
    }
    Ok(grouped.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECTIONS: &str = include_str!("../../../fixtures/relic_projections.json");

    fn projections() -> Vec<Relic> {
        parse_relics(PROJECTIONS).unwrap()
    }

    #[test]
    fn split_refinement_suffix() {
        assert_eq!(
            split_refinement("Axi A1 Intact"),
            Some(("Axi A1", Refinement::Intact))
        );
        assert_eq!(
            split_refinement("Axi A1 Radiant"),
            Some(("Axi A1", Refinement::Radiant))
        );
        assert_eq!(split_refinement("Axi A1"), None);
    }

    #[test]
    fn reward_market_slug() {
        let relics = parse_relics(include_str!("../tests/fixtures/relics.json")).unwrap();
        let with_slug_relic = relics.iter().find(|relic| relic.name == "Axi A1").unwrap();
        let braton_stock = with_slug_relic
            .rewards_for(Refinement::Intact)
            .iter()
            .find(|reward| reward.item_name == "Braton Prime Stock")
            .unwrap();
        assert_eq!(
            braton_stock.market_slug.as_deref(),
            Some("braton_prime_stock")
        );

        let without_slug_relic = relics.iter().find(|relic| relic.name == "Axi A21").unwrap();
        let forma = without_slug_relic
            .rewards_for(Refinement::Intact)
            .iter()
            .find(|reward| reward.item_name == "Forma Blueprint")
            .unwrap();
        assert_eq!(forma.market_slug, None);
    }

    #[test]
    fn drops_best_chance_per_location() {
        let relics = parse_relics(include_str!("../tests/fixtures/relics.json")).unwrap();
        let dropping = relics.iter().find(|relic| relic.name == "Axi A21").unwrap();
        assert!(dropping.drops.len() > 1);
        let mut seen = std::collections::HashSet::new();
        assert!(
            dropping
                .drops
                .iter()
                .all(|drop| seen.insert(drop.location.as_str())),
            "duplicated location"
        );
        assert!(
            dropping
                .drops
                .windows(2)
                .all(|pair| pair[0].chance >= pair[1].chance)
        );
        let plague_star = dropping
            .drops
            .iter()
            .find(|drop| drop.location.contains("Plague Star"))
            .expect("plague star");
        assert!((plague_star.chance - 0.67).abs() < f64::EPSILON);

        let vaulted = relics.iter().find(|relic| relic.name == "Axi A1").unwrap();
        assert!(vaulted.drops.is_empty());
    }

    #[test]
    fn relic_name_placeholders() {
        assert_eq!(relic_name("Lith G12 Intact"), Some("Lith G12"));
        assert_eq!(relic_name("Requiem Eterna Relic"), Some("Requiem Eterna"));
        assert_eq!(relic_name("Lith Relic"), None);
        assert_eq!(relic_name("Void Relic"), None);
    }

    #[test]
    fn projection_suffixes() {
        assert_eq!(
            projection_refinement("T1VoidProjectionWispPrimeABronze"),
            Refinement::Intact
        );
        assert_eq!(
            projection_refinement("T1VoidProjectionWispPrimeAPlatinum"),
            Refinement::Radiant
        );
        assert_eq!(
            projection_refinement("T5VoidProjectionImmortalOmniA"),
            Refinement::Intact
        );
        assert_eq!(
            projection_family("T1VoidProjectionWispPrimeAGold"),
            "T1VoidProjectionWispPrimeA"
        );
    }

    #[test]
    fn same_name_two_families() {
        let relics = projections();
        let named: Vec<&Relic> = relics
            .iter()
            .filter(|relic| relic.name == "Lith G12")
            .collect();
        assert_eq!(named.len(), 2);
        for family in ["SevagothPrimeD", "SevagothPrimeE"] {
            let unique_name =
                format!("/Lotus/Types/Game/Projections/T1VoidProjection{family}Bronze");
            assert!(
                named.iter().any(|relic| {
                    relic.unique_names.get(&Refinement::Intact) == Some(&unique_name)
                }),
                "{unique_name} is not addressable"
            );
        }
    }

    #[test]
    fn requiem_relic() {
        let relics = projections();
        let eterna = relics
            .iter()
            .find(|relic| relic.name == "Requiem Eterna")
            .unwrap();
        assert_eq!(eterna.tier, "Requiem");
        assert!(!eterna.tradable);
        assert_eq!(
            eterna
                .unique_names
                .get(&Refinement::Intact)
                .map(String::as_str),
            Some("/Lotus/Types/Game/Projections/T5VoidProjectionImmortalOmniA")
        );
        for placeholder in ["Lith", "Meso", "Neo", "Axi", "Requiem", "Void"] {
            assert!(
                !relics.iter().any(|relic| relic.name == placeholder),
                "{placeholder} placeholder is catalogued as a relic"
            );
        }
    }
}
