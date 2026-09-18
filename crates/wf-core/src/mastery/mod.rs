mod intrinsics;
mod items;
mod routes;
mod star_chart;
mod tab;
mod xp;

use serde::{Deserialize, Serialize};

pub(crate) use items::{includes_founders, kind_of, masterable, prime_ownership, unmastered_types};
pub(crate) use tab::tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MasteryOrdering {
    #[default]
    Closest,
    FromRelics,
    ByPlatinum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MasteryOptions {
    pub ordering: MasteryOrdering,
    pub include_founders_items: Option<bool>,
    pub include_forma_ranks: bool,
}

impl Default for MasteryOptions {
    fn default() -> Self {
        Self {
            ordering: MasteryOrdering::Closest,
            include_founders_items: None,
            include_forma_ranks: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MasteryGroup {
    Warframes,
    Weapons,
    Companions,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MasteryComponent {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub owned: i64,
    pub required: i64,
    pub enough: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Level {
    pub current: u32,
    pub max: u32,
    pub xp_remaining: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Acquisition {
    pub missing_parts: usize,
    pub plat_cost: u64,
    pub purchasable: bool,
    pub relic_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MasteryItem {
    pub unique_name: String,
    pub name: String,
    pub kind: &'static str,
    pub group: MasteryGroup,
    pub image_name: Option<String>,
    pub owned: bool,
    pub mastered: bool,
    pub level: Level,
    pub acquisition: Acquisition,
    pub favourite: bool,
    pub components: Vec<MasteryComponent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CategoryTotals {
    pub current: u32,
    pub max: u32,
    pub percent: f64,
}

impl CategoryTotals {
    fn new(current: u32, max: u32) -> Self {
        Self {
            current,
            max,
            percent: percent(u64::from(current), u64::from(max)),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "catalog and star chart counts fit in u32"
    )]
    fn counted(current: usize, max: usize) -> Self {
        Self::new(current as u32, max as u32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MasterySummary {
    pub warframes: CategoryTotals,
    pub weapons: CategoryTotals,
    pub companions: CategoryTotals,
    pub star_normal: CategoryTotals,
    pub star_steel: CategoryTotals,
    pub star_junctions: CategoryTotals,
    pub star_steel_junctions: CategoryTotals,
    pub intrinsic_railjack: CategoryTotals,
    pub intrinsic_duviri: CategoryTotals,
    pub content_percent: f64,
    pub star_percent: f64,
    pub intrinsic_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteMember {
    pub name: String,
    pub detail: String,
    pub xp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LevelUpRoute {
    pub kind: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub count: u32,
    pub xp_available: u64,
    pub members: Vec<RouteMember>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MasteryTab {
    pub rank: u32,
    pub founder: bool,
    pub include_founders: bool,
    pub percent: u32,
    pub rank_xp_earned: u64,
    pub rank_xp_span: u64,
    pub summary: MasterySummary,
    pub plat_total: u64,
    pub favourite_xp: u64,
    pub favourite_percent: f64,
    pub recommended: Vec<MasteryItem>,
    pub routes: Vec<LevelUpRoute>,
}

#[allow(
    clippy::cast_precision_loss,
    reason = "mastery XP stays far below 2^53"
)]
fn percent(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    (100.0 * (numerator as f64 / denominator as f64)).round()
}

#[cfg(test)]
mod support {
    use wf_inventory::Inventory;

    use super::MasteryOptions;
    use crate::catalog::fixtures;
    use crate::prices::FixedPrices;

    pub(super) fn prices() -> FixedPrices {
        FixedPrices::new([("trinity_prime_systems_blueprint", 14.0)])
    }

    pub(super) fn founder_inventory() -> Inventory {
        Inventory::parse(&fixtures::INVENTORY.replacen('{', r#"{"Accolades":{"Founder":4},"#, 1))
            .unwrap()
    }

    pub(super) fn excluding_founders() -> MasteryOptions {
        MasteryOptions {
            include_founders_items: Some(false),
            ..MasteryOptions::default()
        }
    }

    pub(super) fn mutated(edit: impl FnOnce(&mut serde_json::Value)) -> Inventory {
        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        edit(&mut value);
        Inventory::parse(&value.to_string()).unwrap()
    }

    pub(super) fn entries<'a>(
        value: &'a mut serde_json::Value,
        key: &str,
    ) -> &'a mut Vec<serde_json::Value> {
        value
            .get_mut(key)
            .and_then(serde_json::Value::as_array_mut)
            .unwrap()
    }

    pub(super) fn drop_affinity(value: &mut serde_json::Value, unique_name: &str) {
        entries(value, "XPInfo").retain(|entry| {
            entry.get("ItemType").and_then(serde_json::Value::as_str) != Some(unique_name)
        });
    }

    pub(super) fn set_affinity(value: &mut serde_json::Value, unique_name: &str, xp: u64) {
        for entry in entries(value, "XPInfo") {
            if entry.get("ItemType").and_then(serde_json::Value::as_str) == Some(unique_name) {
                entry["XP"] = serde_json::json!(xp);
            }
        }
    }

    pub(super) fn mission<'a>(
        value: &'a mut serde_json::Value,
        tag: &str,
    ) -> &'a mut serde_json::Value {
        entries(value, "Missions")
            .iter_mut()
            .find(|entry| entry.get("Tag").and_then(serde_json::Value::as_str) == Some(tag))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_percentages() {
        assert!((percent(0, 0) - 0.0).abs() < f64::EPSILON);
        assert!((percent(1, 2) - 50.0).abs() < f64::EPSILON);
        assert!((percent(1, 3) - 33.0).abs() < f64::EPSILON);
        assert!((percent(2, 3) - 67.0).abs() < f64::EPSILON);
    }
}
