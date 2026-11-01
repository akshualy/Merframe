mod active;
mod boosters;
mod recorder;
mod report;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(test)]
pub(crate) use recorder::fixture_runs;
pub(crate) use recorder::{ArbitrationRunRecorder, ArbitrationRunUpdate};
pub use report::{svg_report, text_report};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ArbitrationRunStatus {
    Recording,
    Completed,
    Failed,
    Aborted,
    Ended,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ArbitrationRunSource {
    Live,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArbitrationRunRotation {
    pub index: usize,
    pub at_secs: f64,
    pub duration_secs: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArbitrationRunWave {
    pub index: u32,
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArbitrationRunSample {
    pub secs: f64,
    pub drone_spawns: u32,
    pub live_agents: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ArbitrationRunResourceBoosterKind {
    Amount,
    DropChance,
    DropChanceBlessing,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArbitrationRunResourceBooster {
    pub kind: ArbitrationRunResourceBoosterKind,
    pub expires_secs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArbitrationRun {
    pub id: String,
    pub source: ArbitrationRunSource,
    pub status: ArbitrationRunStatus,
    pub node_id: Option<String>,
    pub node: Option<String>,
    pub planet: Option<String>,
    pub mission_type: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub duration_secs: f64,
    pub host: Option<bool>,
    pub complete_capture: bool,
    pub notes: Vec<String>,
    pub rotations: Vec<ArbitrationRunRotation>,
    pub waves: Vec<ArbitrationRunWave>,
    pub reward_cycles: Option<usize>,
    pub enemy_spawns: Option<u32>,
    pub drone_spawns: Option<u32>,
    pub peak_live_agents: Option<u32>,
    pub boosters: Vec<ArbitrationRunResourceBooster>,
    pub samples: Vec<ArbitrationRunSample>,
}

impl ArbitrationRun {
    pub fn place(&self) -> String {
        match (&self.node, &self.planet) {
            (Some(node), Some(planet)) => format!("{node} ({planet})"),
            _ => String::from("Unknown node"),
        }
    }
}
