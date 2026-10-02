use std::sync::Arc;

use chrono::{DateTime, Utc};
use tauri::State;
use wf_core::TimeRange;

use crate::error::{CommandError, CommandResult};
use crate::state::{AppState, AppStateCell, lock};

pub mod analytics;
pub mod app;
pub mod market;
pub mod tabs;
pub mod world;

pub use world::WorldStateView;

type Shared<'a> = State<'a, AppStateCell>;

async fn ready(cell: &AppStateCell) -> CommandResult<Arc<AppState>> {
    cell.wait().await.clone().map_err(CommandError::from)
}

fn missing_inventory() -> CommandError {
    CommandError::from("No inventory has been read from the game yet")
}

fn since(since_ms: Option<i64>) -> TimeRange {
    match since_ms.and_then(DateTime::<Utc>::from_timestamp_millis) {
        Some(from) => TimeRange::since(from),
        None => TimeRange::all(),
    }
}

async fn compute<T, F>(state: Arc<AppState>, build: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce(&wf_core::Core) -> Option<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || build(&lock(&state.core)))
        .await?
        .ok_or_else(missing_inventory)
}
