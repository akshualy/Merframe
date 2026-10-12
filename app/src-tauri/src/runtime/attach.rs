use std::sync::Arc;

use anyhow::Context;
use wf_mem::{MemoryReader, open_pid};
use wf_scan::{LuaState, LuaTable};

use super::blocking;
use crate::state::{AppState, read};

pub(super) struct Attached {
    pub(super) reader: Arc<dyn MemoryReader>,
    pub(super) lua: LuaState,
}

impl Attached {
    pub(super) fn holds(&self) -> bool {
        self.lua.holds(self.reader.as_ref())
    }
}

pub(super) fn game_pid(state: &AppState) -> anyhow::Result<u32> {
    read(&state.status).pid.context("The game is not running")
}

pub(crate) fn open_game(state: &AppState) -> anyhow::Result<Box<dyn MemoryReader>> {
    open_pid(game_pid(state)?).context("Attaching to the game process")
}

pub(super) fn attach(state: &AppState) -> anyhow::Result<Attached> {
    let reader: Arc<dyn MemoryReader> = Arc::from(open_game(state)?);
    let lua = LuaState::locate(reader.as_ref())?.context("The game's Lua state was not found")?;
    Ok(Attached { reader, lua })
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
