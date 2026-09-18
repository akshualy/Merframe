use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::catalog::VaultStatus;

mod stock;
mod tab;
mod tree;

pub(crate) use tab::tab;
pub(crate) use tree::details;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PendingBuild {
    pub item_type: String,
    pub name: String,
    pub image_name: Option<String>,
    pub completes_at: DateTime<Utc>,
    pub remaining_secs: i64,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissingComponent {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub owned: i64,
    pub required: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CraftNode {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub required: i64,
    pub owned: i64,
    pub per_craft: i64,
    pub short_by: i64,
    pub crafts_queued: i64,
    pub craftable: bool,
    pub stocked: bool,
    pub covered: bool,
    pub children: Vec<CraftNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NeededItem {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundryComponent {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub owned: i64,
    pub required: i64,
    pub enough: bool,
    pub favourite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Prime {
    pub vault: VaultStatus,
    pub resurgence: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub owned: bool,
    pub pending: bool,
    pub ready_to_build: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MasteryGate {
    pub required: Option<u32>,
    pub met: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Helminth {
    pub ability: String,
    pub subsumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundryItem {
    pub unique_name: String,
    pub name: String,
    pub kind: &'static str,
    pub type_name: String,
    pub image_name: Option<String>,
    pub prime: Option<Prime>,
    pub mastered: bool,
    pub progress: Progress,
    pub crafts_into: Vec<String>,
    pub mastery: MasteryGate,
    pub incarnon: bool,
    pub helminth: Option<Helminth>,
    pub archon_shards: u32,
    pub favourite: bool,
    pub components: Vec<FoundryComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorldTimer {
    pub name: String,
    pub state: String,
    pub ends_at: DateTime<Utc>,
    pub remaining_secs: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CraftDetails {
    pub tree: Vec<CraftNode>,
    pub missing: Vec<MissingComponent>,
    pub summary: CraftSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CraftSummary {
    pub credits: i64,
    pub build_secs: i64,
    pub shortest_secs: i64,
    pub blueprints_needed: Vec<NeededItem>,
    pub resources_needed: Vec<NeededItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FoundryTab {
    pub pending: Vec<PendingBuild>,
    pub items: Vec<FoundryItem>,
    pub timers: Vec<WorldTimer>,
}
