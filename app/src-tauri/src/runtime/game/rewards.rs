use std::mem::DropGuard;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use chrono::Utc;
use tauri::{AppHandle, Runtime};
use tracing::{debug, warn};
use wf_core::ScannedRewards;
use wf_mem::MemoryReader;
use wf_scan::{LuaState, LuaTable, RelicPicker};

use super::core_events;
use crate::runtime::attach::{Attached, attach_or_warn, read_screen};
use crate::state::{AppState, lock};

const SCREEN_TICK: Duration = Duration::from_millis(100);
const REWARD_TICKS: usize = 50;
const REWARD_PICK_LIFETIME: Duration = Duration::from_secs(60);
const REWARD_PICK_GRACE: Duration = Duration::from_secs(1);
const PICKER_LIFETIME: Duration = Duration::from_secs(600);
const PICKER_GRACE: Duration = Duration::from_secs(2);

pub(super) async fn handle_reward_screen<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let generation = lock(&state.core).reward_screen_generation();
    let Some(attached) = attach_or_warn(state, "Reward screen read from game memory failed").await
    else {
        return;
    };

    let Some(screen) = scan_reward_screen(state, &attached).await else {
        return;
    };

    let handled = core_events(app, state, "scanned rewards", move |core| {
        core.handle_reward_screen(Utc::now(), generation, screen)
    })
    .await;
    if let Err(error) = handled {
        warn!(%error, generation, "Core rejected the scanned rewards");
        return;
    }

    let mut last = None;
    watch_screen(
        "read reward pick",
        &attached,
        REWARD_PICK_LIFETIME,
        REWARD_PICK_GRACE,
        wf_scan::picked_relic_reward,
        |pick| {
            if last.as_ref() != Some(&pick) {
                debug!(
                    item_type = pick.item_type,
                    count = pick.count,
                    "Relic reward picked"
                );
                lock(&state.core).keep_relic_reward_pick(&pick.item_type, pick.count);
                last = Some(pick);
            }
        },
    )
    .await;
}

async fn watch_screen<T, F>(
    task: &'static str,
    attached: &Attached,
    lifetime: Duration,
    grace: Duration,
    read: F,
    mut seen: impl FnMut(T),
) where
    T: Send + 'static,
    F: Fn(&(dyn MemoryReader + 'static), &LuaState, &mut Option<LuaTable>) -> Option<T>
        + Copy
        + Send
        + 'static,
{
    let started = Instant::now();
    let mut gone_since = None;
    let mut script = None;
    while started.elapsed() < lifetime {
        let screen = read_screen(task, &attached.reader, attached.lua, &mut script, read).await;
        if let Some(value) = screen.flatten() {
            gone_since = None;
            seen(value);
        } else {
            let since = *gone_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= grace {
                break;
            }
        }

        tokio::time::sleep(SCREEN_TICK).await;
    }
}

pub(super) fn watch_relic_picker(state: &Arc<AppState>) {
    if state.watching_picker.swap(true, Ordering::SeqCst) {
        return;
    }

    let state = DropGuard::new(Arc::clone(state), |state| {
        state.watching_picker.store(false, Ordering::SeqCst);
    });
    tauri::async_runtime::spawn(async move {
        let Some(attached) = attach_or_warn(
            &state,
            "Relic pick not recorded since the picker was not found in memory",
        )
        .await
        else {
            return;
        };

        watch_screen(
            "read relic picker",
            &attached,
            PICKER_LIFETIME,
            PICKER_GRACE,
            |reader, lua, script| match RelicPicker::read(reader, lua, script) {
                RelicPicker::Open(pick) => Some(pick),
                RelicPicker::Closed => None,
            },
            |pick| {
                if let Some(pick) = pick
                    && lock(&state.relic_picks)
                        .insert(pick.handle, pick.relic_type.clone())
                        .is_none()
                {
                    debug!(relic_type = pick.relic_type, "Relic selected in the picker");
                }
            },
        )
        .await;
    });
}

fn reward_screen(
    state: &AppState,
    reader: &dyn MemoryReader,
    lua: &LuaState,
    script: &mut Option<LuaTable>,
) -> ScannedRewards {
    let tiles = wf_scan::reward_screen(reader, lua, script);
    let own_relic_type = wf_scan::confirmed_relic(reader, lua)
        .and_then(|handle| lock(&state.relic_picks).get(&handle).cloned());
    ScannedRewards {
        own_relic_type,
        rewards: tiles.into_iter().map(|tile| tile.store_item).collect(),
    }
}

async fn scan_reward_screen(state: &Arc<AppState>, attached: &Attached) -> Option<ScannedRewards> {
    let mut seen = ScannedRewards::default();
    let mut script = None;
    for _ in 0..REWARD_TICKS {
        let owned = Arc::clone(state);
        let screen = read_screen(
            "read reward screen",
            &attached.reader,
            attached.lua,
            &mut script,
            move |reader, lua, script| reward_screen(&owned, reader, lua, script),
        )
        .await?;
        if !screen.rewards.is_empty() && screen == seen {
            if screen.own_relic_type.is_none() {
                debug!("Reward screen settled without a known relic for the local player");
            }
            return Some(screen);
        }
        seen = screen;
        tokio::time::sleep(SCREEN_TICK).await;
    }
    debug!("Reward screen held no settled tiles");
    None
}
