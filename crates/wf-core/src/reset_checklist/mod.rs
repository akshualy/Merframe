mod cadence;
mod context;
mod definitions;
mod nightwave;
mod standing;
mod syndicate_missions;
mod trackers;
mod unlocks;
mod vendors;

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Serialize;
use wf_worldstate::ResetWindow;

use cadence::ResetCadence;
pub use cadence::ResetChecklistGroup;
pub(crate) use context::{ResetChecklistContext, ResetChecklistWorld};
use definitions::{RESET_CHECKLIST_ROWS, ResetChecklistRowDefinition};
pub use definitions::{ResetChecklistManualRowId, ResetChecklistRowId};
pub use standing::ResetChecklistStandingPool;
pub use trackers::ResetChecklistProgress;
use trackers::Tracker;
pub use vendors::{ArchonShardColour, KuvaWeeklySummary, ResetChecklistVendor, VendorWeeklyOffer};

use crate::error::Result;
use crate::store::Store;

pub(crate) const MISSION_WON_SETTING: &str = "reset_checklist_mission_won_at";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ResetChecklistRow {
    pub id: ResetChecklistRowId,
    pub name: &'static str,
    pub group: ResetChecklistGroup,
    pub progress: ResetChecklistProgress,
    pub done: bool,
    pub locked: bool,
    pub own_window: Option<ResetWindow>,
    pub detail: Option<String>,
    pub parts: Vec<ResetChecklistRowPart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ResetChecklistRowPart {
    pub name: String,
    pub done: u32,
    pub total: u32,
    pub completed: bool,
}

impl ResetChecklistRowPart {
    pub(crate) fn new(name: String, done: u32, total: u32) -> Self {
        Self {
            name,
            done,
            total,
            completed: done >= total,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ResetChecklistTab {
    pub daily: ResetWindow,
    pub weekly: ResetWindow,
    pub sortie: Option<ResetWindow>,
    pub synced_at: Option<DateTime<Utc>>,
    pub rows: Vec<ResetChecklistRow>,
    pub standing: Vec<ResetChecklistStandingPool>,
    pub vendors: Vec<ResetChecklistVendor>,
    pub kuva: KuvaWeeklySummary,
}

fn reset_checklist_row(
    definition: &ResetChecklistRowDefinition,
    context: &ResetChecklistContext,
    completed_quests: &HashSet<&str>,
) -> ResetChecklistRow {
    let cadence = definition.tracker.cadence();
    let tracked = definition.tracker.track(context);

    ResetChecklistRow {
        id: definition.id,
        name: definition.name,
        group: cadence.group(),
        progress: tracked.progress,
        done: tracked.done,
        locked: unlocks::locked(definition.requirements, context.inventory, completed_quests),
        own_window: cadence.own_window(context),
        detail: tracked.detail,
        parts: tracked.parts,
    }
}

pub(crate) fn tab(context: &ResetChecklistContext) -> ResetChecklistTab {
    let completed_quests = context.inventory.completed_quests();
    let vendors = vendors::vendors(context);

    ResetChecklistTab {
        daily: ResetCadence::Daily.window(context.now),
        weekly: ResetCadence::Weekly.window(context.now),
        sortie: ResetChecklistGroup::SortieReset.reset(context),
        synced_at: context.synced,
        rows: RESET_CHECKLIST_ROWS
            .iter()
            .map(|definition| reset_checklist_row(definition, context, &completed_quests))
            .collect(),
        standing: standing::standing_pools(context),
        kuva: vendors::kuva_summary(&vendors),
        vendors,
    }
}

fn manual_rows() -> impl Iterator<Item = (ResetChecklistManualRowId, ResetCadence)> {
    RESET_CHECKLIST_ROWS
        .iter()
        .filter_map(|definition| match definition.tracker {
            Tracker::Manual { id, cadence } => Some((id, cadence)),
            Tracker::Inventory(_) | Tracker::World(_) => None,
        })
}

pub(crate) fn checked_manual_rows(
    store: &Store,
    now: DateTime<Utc>,
) -> Result<HashSet<ResetChecklistManualRowId>> {
    manual_rows()
        .filter_map(|(id, cadence)| {
            store
                .reset_checklist_ticked(id, cadence.window(now).activation)
                .map(|ticked| ticked.then_some(id))
                .transpose()
        })
        .collect()
}

pub(crate) fn set_checked(
    store: &Store,
    id: ResetChecklistManualRowId,
    checked: bool,
    now: DateTime<Utc>,
) -> Result<()> {
    manual_rows()
        .find(|&(manual, _)| manual == id)
        .map_or(Ok(()), |(id, cadence)| {
            store.set_reset_checklist_ticked(id, cadence.window(now).activation, checked)
        })
}

#[cfg(test)]
mod tests;
