mod alerts;
mod engine;

use chrono::{DateTime, Utc};
use serde::Serialize;

pub use alerts::{
    AlertSettings, CyclePhase, FissureFilter, SteelPathFilter, TimerAlerts, tier_name,
};
pub(crate) use engine::{Engine, inventory_events};

use crate::trade::Trade;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FissureInfo {
    pub node_id: String,
    pub node_name: Option<&'static str>,
    pub mission_type: String,
    pub mission_name: String,
    pub planet: Option<&'static str>,
    pub tier: String,
    pub steel_path: bool,
    pub expiry: DateTime<Utc>,
    pub remaining_secs: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventorySummary {
    pub last_sync_oid: String,
    pub mr: u32,
    pub plat: i64,
    pub credits: i64,
    pub endo: i64,
    pub ducats: i64,
    pub changes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CoreEvent {
    InventoryUpdated(InventorySummary),
    RelicRewardScreen {
        relic: Option<String>,
        rewards: Vec<String>,
    },
    TradeCompleted {
        at: DateTime<Utc>,
        partner: Option<String>,
        trade: Trade,
    },
    NewConversation {
        channel: String,
        player: String,
    },
    FissureAlert {
        fissure: FissureInfo,
    },
    TimerAlert {
        name: String,
        next_state: String,
        ends_at: DateTime<Utc>,
        remaining_secs: i64,
    },
}

impl CoreEvent {
    pub fn label(&self) -> &'static str {
        match self {
            Self::InventoryUpdated(_) => "InventoryUpdated",
            Self::RelicRewardScreen { .. } => "RelicRewardScreen",
            Self::TradeCompleted { .. } => "TradeCompleted",
            Self::NewConversation { .. } => "NewConversation",
            Self::FissureAlert { .. } => "FissureAlert",
            Self::TimerAlert { .. } => "TimerAlert",
        }
    }
}
