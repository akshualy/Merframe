use std::sync::Arc;

use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::state::{AppState, AppStateCell, lock};

pub mod app;
pub mod market;
pub mod tabs;
pub mod world;

pub use world::WorldStateView;

type Shared<'a> = State<'a, AppStateCell>;

fn ready(cell: &AppStateCell) -> CommandResult<Arc<AppState>> {
    if let Some(state) = cell.get() {
        return Ok(Arc::clone(state));
    }
    tracing::debug!("Command arrived before app state was published");
    Err(CommandError::starting())
}

fn missing_inventory() -> CommandError {
    CommandError::from("No inventory has been read from the game yet")
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
