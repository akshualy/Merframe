use chrono::{DateTime, Utc};
use tauri::{AppHandle, Runtime};
use wf_core::{
    ComparedStat, CraftDetails, FoundryTab, InventoryTab, MasteryOptions, MasteryOrdering,
    MasteryTab, RelicPlannerTab, RelicSource, ResourcesTab, RewardScreen, RivenComparables,
    RivensTab, StatsTab, TimeRange,
};

use super::{Shared, compute, ready};
use crate::error::CommandResult;
use crate::runtime;
use crate::state::{lock, read};

#[tauri::command]
pub async fn inventory_tab(state: Shared<'_>) -> CommandResult<InventoryTab> {
    let state = ready(&state)?;
    compute(state, wf_core::Core::inventory_tab).await
}

#[tauri::command]
pub async fn foundry_tab(state: Shared<'_>) -> CommandResult<FoundryTab> {
    let state = ready(&state)?;
    let include_founders = read(&state.settings)
        .mastery_options(MasteryOrdering::default())
        .include_founders_items;
    compute(state, move |core| {
        core.foundry_tab(include_founders, Utc::now())
    })
    .await
}

#[tauri::command]
pub async fn craft_tree(state: Shared<'_>, unique_name: String) -> CommandResult<CraftDetails> {
    let state = ready(&state)?;
    compute(state, move |core| core.craft_tree(&unique_name)).await
}

#[tauri::command]
pub async fn mastery_tab(
    state: Shared<'_>,
    ordering: Option<MasteryOrdering>,
    include_founders: Option<bool>,
    include_forma_ranks: Option<bool>,
) -> CommandResult<MasteryTab> {
    let state = ready(&state)?;
    let stored = read(&state.settings).mastery_options(ordering.unwrap_or_default());
    let options = MasteryOptions {
        ordering: stored.ordering,
        include_founders_items: include_founders.or(stored.include_founders_items),
        include_forma_ranks: include_forma_ranks.unwrap_or(stored.include_forma_ranks),
    };
    compute(state, move |core| core.mastery_tab(options)).await
}

#[tauri::command]
pub async fn resources_tab(state: Shared<'_>) -> CommandResult<ResourcesTab> {
    let state = ready(&state)?;
    compute(state, wf_core::Core::resources_tab).await
}

#[tauri::command]
pub async fn relic_planner_tab(
    state: Shared<'_>,
    squad_size: Option<u32>,
    only_owned: Option<bool>,
) -> CommandResult<RelicPlannerTab> {
    let state = ready(&state)?;
    let squad = squad_size.unwrap_or(wf_core::DEFAULT_SQUAD_SIZE);
    let owned = only_owned.unwrap_or(true);
    compute(state, move |core| core.relic_planner_tab(squad, owned)).await
}

#[tauri::command]
pub async fn rivens_tab(state: Shared<'_>) -> CommandResult<RivensTab> {
    let state = ready(&state)?;
    compute(state, wf_core::Core::rivens_tab).await
}

#[tauri::command]
pub async fn riven_comparables(
    state: Shared<'_>,
    weapon_slug: String,
    shown: Vec<ComparedStat>,
) -> CommandResult<Option<RivenComparables>> {
    let state = ready(&state)?;
    let Some(auctions) = state.auctions.weapon(&state.http, &weapon_slug).await? else {
        return Ok(None);
    };
    Ok(Some(lock(&state.core).riven_comparables(&shown, &auctions)))
}

#[tauri::command]
pub async fn stats_tab(state: Shared<'_>, since_ms: Option<i64>) -> CommandResult<StatsTab> {
    let state = ready(&state)?;
    let range = match since_ms.and_then(DateTime::from_timestamp_millis) {
        Some(from) => TimeRange::since(from),
        None => TimeRange::all(),
    };
    Ok(lock(&state.core).stats_tab(range, Utc::now())?)
}

#[tauri::command]
pub async fn toggle_favourite<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    unique_name: String,
) -> CommandResult<bool> {
    let state = ready(&state)?;
    let favourite = lock(&state.core).toggle_favourite(&unique_name)?;
    runtime::emit(&app, runtime::INVENTORY_UPDATED, state.status_snapshot());
    Ok(favourite)
}

#[tauri::command]
pub async fn relics_for(
    state: Shared<'_>,
    part_unique_name: String,
) -> CommandResult<Vec<RelicSource>> {
    let state = ready(&state)?;
    compute(state, move |core| core.relics_for(&part_unique_name)).await
}

#[tauri::command]
pub async fn recommend(state: Shared<'_>, rewards: Vec<String>) -> CommandResult<RewardScreen> {
    let state = ready(&state)?;
    Ok(tauri::async_runtime::spawn_blocking(move || lock(&state.core).recommend(&rewards)).await?)
}
