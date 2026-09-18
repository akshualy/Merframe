use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::Utc;
use futures_util::StreamExt;
use futures_util::pin_mut;
use tauri::{AppHandle, Runtime};
use tracing::{debug, warn};
use wf_core::CoreEvent;
use wf_log::Event as LogEvent;
use wf_mem::{GAME_PROCESS, MemError, MemoryReader, open_game};
use wf_scan::{HttpClients, InventoryBuffer};

use super::{INVENTORY_UPDATED, STATUS_UPDATED, blocking, dispatch, emit};
use crate::overlay;
use crate::state::{AppState, InventorySource, QueueSession, lock, read, write};

const PROCESS_POLL: Duration = Duration::from_secs(5);
const REWARD_TICK: Duration = Duration::from_millis(100);
const REWARD_TICKS: usize = 50;

pub(super) async fn log_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let Some(path) = wf_log::default_log_path() else {
        warn!("No EE.log path on this platform");
        return;
    };
    write(&state.status).log_file = Some(path.display().to_string());
    emit(&app, STATUS_UPDATED, state.status_snapshot());

    loop {
        let selection = read(&state.settings).log_selection();
        match wf_log::lines(&path, selection) {
            Ok(stream) => {
                write(&state.status).log_attached = true;
                emit(&app, STATUS_UPDATED, state.status_snapshot());
                pin_mut!(stream);
                while let Some(line) = stream.next().await {
                    match line {
                        Ok(source) => {
                            if let Some(event) = wf_log::classify(&source.line) {
                                let started = Instant::now();
                                debug!(
                                    event = event.label(),
                                    origin = source.origin.label(),
                                    at = source.line.time,
                                    "Log event"
                                );
                                handle_log_event(&app, &state, event, source.line.time).await;
                                if started.elapsed() > Duration::from_secs(1) {
                                    warn!(
                                        millis = started.elapsed().as_millis(),
                                        "Log event handled slowly"
                                    );
                                }
                            }
                        }
                        Err(error) => warn!(%error, "EE.log line was unreadable, skipped"),
                    }
                }
                write(&state.status).log_attached = false;
                emit(&app, STATUS_UPDATED, state.status_snapshot());
                warn!("Log stream ended, reattaching in 10 s");
            }
            Err(error) => warn!(%error, "Opening EE.log failed, next attempt in 10 s"),
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

pub(super) async fn process_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    loop {
        let found = match blocking("find game process", || wf_mem::find_process(GAME_PROCESS)).await
        {
            Some(Ok(pid)) => Some(pid),
            Some(Err(MemError::ProcessNotFound(_))) | None => None,
            Some(Err(error)) => {
                warn!(
                    %error,
                    "Process list was unreadable, treating the game as not running",
                );
                None
            }
        };
        let changed = {
            let mut status = write(&state.status);
            let changed = status.game_detected != found.is_some() || status.pid != found;
            status.game_detected = found.is_some();
            status.pid = found;
            changed
        };
        if changed {
            debug!(
                pid = found,
                detected = found.is_some(),
                "Game process changed"
            );
            emit(&app, STATUS_UPDATED, state.status_snapshot());
            if found.is_some() {
                state.rescan.notify_one();
            } else {
                state.focus.forget();
            }
        }
        tokio::time::sleep(PROCESS_POLL).await;
    }
}

async fn handle_log_event<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    event: LogEvent,
    at: f64,
) {
    if let LogEvent::WindowFocus { focused } = &event {
        state.focus.changed(*focused);
    }
    overlay::on_log_event(app, state, &event, at);
    if matches!(event, LogEvent::InventorySynced) {
        capture(app, state);
    }
    let scan_rewards = matches!(
        event,
        LogEvent::SquadRewardInfoComplete | LogEvent::RelicRewardsShown
    );
    let owned = Arc::clone(state);
    let handled = blocking("handle log event", move || {
        let mut core = lock(&owned.core);
        core.handle_log_event(&event, Utc::now())
    })
    .await;
    match handled {
        Some(Ok(events)) => dispatch(app, state, events).await,
        Some(Err(error)) => warn!(%error, "Core rejected log event"),
        None => {}
    }
    let started = scan_rewards && lock(&state.core).start_reward_scan();
    if started {
        let app = app.clone();
        let state = Arc::clone(state);
        tauri::async_runtime::spawn(async move { handle_reward_screen(&app, &state).await });
    }
}

async fn handle_reward_screen<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let generation = lock(&state.core).reward_screen_generation();
    let Some(rewards) = scan_reward_screen().await else {
        return;
    };
    let owned = Arc::clone(state);
    let handled = blocking("handle reward screen", move || {
        lock(&owned.core).handle_reward_screen(Utc::now(), generation, rewards)
    })
    .await;
    match handled {
        Some(Ok(events)) => dispatch(app, state, events).await,
        Some(Err(error)) => {
            warn!(%error, generation, "Core rejected the scanned rewards");
        }
        None => {}
    }
}

fn reward_screen() -> anyhow::Result<Vec<String>> {
    let reader = open_game().context("Attaching to the game process")?;
    let tiles = wf_scan::reward_screen(reader.as_ref()).context("Reading the reward screen")?;
    Ok(tiles.into_iter().map(|tile| tile.store_item).collect())
}

async fn scan_reward_screen() -> Option<Vec<String>> {
    let mut seen = Vec::new();
    for _ in 0..REWARD_TICKS {
        let rewards = match blocking("read reward screen", reward_screen).await? {
            Ok(rewards) => rewards,
            Err(error) => {
                warn!(?error, "Reward screen read from game memory failed");
                return None;
            }
        };
        if !rewards.is_empty() && rewards == seen {
            return Some(rewards);
        }
        seen = rewards;
        tokio::time::sleep(REWARD_TICK).await;
    }
    debug!("Reward screen held no settled tiles");
    None
}

pub(super) async fn inventory_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    loop {
        state.rescan.notified().await;
        if read(&state.status).game_detected {
            acquire(&app, &state).await;
        }
    }
}

fn capture<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    if state.capturing.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    let state = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        acquire_with(&app, &state, |state| {
            ingest_window(state, Duration::from_secs(10))
        })
        .await;
        state.capturing.store(false, Ordering::SeqCst);
    });
}

pub async fn acquire<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) -> bool {
    acquire_with(app, state, |state| ingest_window(state, Duration::ZERO)).await
}

async fn acquire_with<R, F>(app: &AppHandle<R>, state: &Arc<AppState>, ingest: F) -> bool
where
    R: Runtime,
    F: FnOnce(&Arc<AppState>) -> anyhow::Result<Ingested> + Send + 'static,
{
    write(&state.status).scanning = true;
    emit(app, STATUS_UPDATED, state.status_snapshot());

    let owned = Arc::clone(state);
    let outcome = tauri::async_runtime::spawn_blocking(move || ingest(&owned))
        .await
        .map_err(anyhow::Error::from)
        .flatten();

    let ingested = match outcome {
        Ok(ingested) => ingested,
        Err(error) => {
            let mut status = write(&state.status);
            status.scanning = false;
            status.last_scan_error = Some(format!("{error:#}"));
            drop(status);
            emit(app, STATUS_UPDATED, state.status_snapshot());
            return false;
        }
    };

    {
        let mut status = write(&state.status);
        status.scanning = false;
        status.last_scan_at = Some(Utc::now());
        status.last_scan_error = None;
        status.source = InventorySource::Live;
        if let Ingested::Newer { last_sync, .. } = &ingested {
            status.last_sync_oid = Some(last_sync.clone());
        }
    }

    emit(app, STATUS_UPDATED, state.status_snapshot());
    match ingested {
        Ingested::Newer { events, .. } => {
            emit(app, INVENTORY_UPDATED, state.status_snapshot());
            overlay::on_inventory_updated(app, state);
            state.prices_wake.notify_one();
            dispatch(app, state, events).await;
        }
        Ingested::Unchanged => {}
    }
    true
}

enum Ingested {
    Newer {
        last_sync: String,
        events: Vec<CoreEvent>,
    },
    Unchanged,
}

fn needs_resolve(session: Option<&QueueSession>, pid: u32) -> bool {
    session.is_none_or(|session| session.pid != pid || session.clients.is_none())
}

fn http_clients(state: &Arc<AppState>, reader: &dyn MemoryReader) -> anyhow::Result<HttpClients> {
    let pid = read(&state.status).pid.context("The game is not running")?;
    let mut session = lock(&state.http_clients);
    if needs_resolve(session.as_ref(), pid) {
        let clients = match HttpClients::locate(reader) {
            Ok(clients) => clients,
            Err(error) => {
                warn!(pid, %error, "Game HTTP client lookup failed");
                None
            }
        };
        *session = Some(QueueSession { pid, clients });
    }
    session
        .as_ref()
        .and_then(|session| session.clients.clone())
        .context("The game's HTTP client was not found")
}

fn ingest_window(state: &Arc<AppState>, window: Duration) -> anyhow::Result<Ingested> {
    let reader = open_game().context("Attaching to the game process")?;
    let clients = http_clients(state, reader.as_ref())?;
    let Some(buffer) = clients.await_inventory(reader.as_ref(), window) else {
        debug!(
            window_millis = window.as_millis(),
            "No inventory response passed through the HTTP queue"
        );
        return Ok(Ingested::Unchanged);
    };
    ingest_buffer(state, buffer)
}

fn ingest_buffer(state: &Arc<AppState>, buffer: InventoryBuffer) -> anyhow::Result<Ingested> {
    debug!(
        addr = format!("{:#x}", buffer.addr),
        len = buffer.len,
        last_sync = buffer.last_sync,
        "Inventory document accepted"
    );
    if let Some(current) = read(&state.status).last_sync_oid.clone()
        && buffer.last_sync <= current
    {
        debug!(
            last_sync = buffer.last_sync,
            current, "Captured inventory is not newer, skipped"
        );
        return Ok(Ingested::Unchanged);
    }
    let mut core = lock(&state.core);
    let events = core
        .ingest_inventory(&buffer.json, Utc::now())
        .context("Ingesting the inventory")?;
    Ok(Ingested::Newer {
        last_sync: buffer.last_sync,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(pid: u32) -> QueueSession {
        QueueSession { pid, clients: None }
    }

    #[test]
    fn resolve_without_clients() {
        let unresolved = session(15556);
        assert!(needs_resolve(None, 15556));
        assert!(needs_resolve(Some(&unresolved), 15556));
        assert!(needs_resolve(Some(&unresolved), 280));
    }
}
