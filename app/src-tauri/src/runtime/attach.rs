use std::sync::Arc;

use anyhow::Context;
use tracing::warn;
use wf_mem::{MemoryReader, open_pid};
use wf_scan::{LuaState, LuaTable};

use super::blocking;
use crate::state::{AppState, read};

pub(crate) struct Attached {
    pub(crate) reader: Arc<dyn MemoryReader>,
    pub(crate) lua: LuaState,
}

impl Attached {
    fn holds(&self) -> bool {
        self.lua.holds(self.reader.as_ref())
    }
}

pub(crate) fn open_game(state: &AppState) -> anyhow::Result<(u32, Box<dyn MemoryReader>)> {
    let pid = read(&state.status).pid.context("The game is not running")?;
    let reader = open_pid(pid).context("Attaching to the game process")?;
    Ok((pid, reader))
}

pub(super) fn attach(state: &AppState) -> anyhow::Result<Attached> {
    let (_, reader) = open_game(state)?;
    let reader: Arc<dyn MemoryReader> = Arc::from(reader);
    let lua = LuaState::locate(reader.as_ref())?.context("The game's Lua state was not found")?;
    Ok(Attached { reader, lua })
}

pub(crate) fn reattach(state: &AppState, cached: Option<Attached>) -> anyhow::Result<Attached> {
    match cached.filter(Attached::holds) {
        Some(attached) => Ok(attached),
        None => attach(state),
    }
}

pub(super) async fn attach_or_warn(
    state: &Arc<AppState>,
    consequence: &'static str,
) -> Option<Attached> {
    let owned = Arc::clone(state);
    match blocking("attach to the game", move || attach(&owned)).await? {
        Ok(attached) => Some(attached),
        Err(error) => {
            warn!(?error, "{consequence}");
            None
        }
    }
}

pub(super) async fn read_screen<T, F>(
    task: &'static str,
    reader: &Arc<dyn MemoryReader>,
    lua: LuaState,
    script: &mut Option<LuaTable>,
    read: F,
) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce(&(dyn MemoryReader + 'static), &LuaState, &mut Option<LuaTable>) -> T
        + Send
        + 'static,
{
    let reader = Arc::clone(reader);
    let mut cached = *script;
    let (value, resolved) = blocking(task, move || {
        let value = read(reader.as_ref(), &lua, &mut cached);
        (value, cached)
    })
    .await?;
    *script = resolved;
    Some(value)
}
