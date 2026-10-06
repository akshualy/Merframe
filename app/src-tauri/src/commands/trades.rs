use chrono::{DateTime, Utc};
use tauri::{AppHandle, Runtime};
use wf_core::Trade;

use super::{Shared, ready};
use crate::error::{CommandError, CommandResult};
use crate::runtime::{self, AppEvent};
use crate::state::lock;

fn trade_time(at_ms: i64) -> CommandResult<DateTime<Utc>> {
    DateTime::from_timestamp_millis(at_ms)
        .ok_or_else(|| CommandError::from("The trade time is out of range"))
}

#[tauri::command]
pub async fn record_trade<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    at_ms: i64,
    partner: Option<String>,
    trade: Trade,
) -> CommandResult<i64> {
    let state = ready(&state).await?;
    let at = trade_time(at_ms)?;
    let id = lock(&state.core)
        .store()
        .record_trade(at, partner.as_deref(), &trade)?;
    runtime::emit(&app, AppEvent::InventoryUpdated(state.status_snapshot()));
    Ok(id)
}

#[tauri::command]
pub async fn update_trade<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    id: i64,
    at_ms: i64,
    partner: Option<String>,
    trade: Trade,
) -> CommandResult<()> {
    let state = ready(&state).await?;
    let at = trade_time(at_ms)?;
    lock(&state.core)
        .store()
        .update_trade(id, at, partner.as_deref(), &trade)?;
    runtime::emit(&app, AppEvent::InventoryUpdated(state.status_snapshot()));
    Ok(())
}

#[tauri::command]
pub async fn delete_trade<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    id: i64,
) -> CommandResult<()> {
    let state = ready(&state).await?;
    lock(&state.core).store().delete_trade(id)?;
    runtime::emit(&app, AppEvent::InventoryUpdated(state.status_snapshot()));
    Ok(())
}
