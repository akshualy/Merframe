use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::Utc;
use futures_util::StreamExt;
use futures_util::pin_mut;
use tauri::{AppHandle, Runtime};
use tracing::{debug, info, warn};
use wf_core::{CoreEvent, ScannedRewards, ScannedTrade, ScannedTradeItem};
use wf_log::Event as LogEvent;
use wf_mem::{GAME_PROCESS, MemError, MemoryReader, open_game};
use wf_scan::{HttpClients, InventoryBuffer, LuaState, TradeScreen};

use super::{AppEvent, blocking, dispatch, emit, market_loop};
use crate::overlay;
use crate::state::{AppState, InventorySource, QueueSession, lock, read, write};

const PROCESS_POLL: Duration = Duration::from_secs(5);
const REWARD_TICK: Duration = Duration::from_millis(100);
const REWARD_TICKS: usize = 50;
const PICKER_TICK: Duration = Duration::from_millis(100);
const PICKER_LIFETIME: Duration = Duration::from_secs(600);
const PICKER_GRACE: Duration = Duration::from_secs(2);
const TRADE_TICK: Duration = Duration::from_millis(50);
const TRADE_WATCH_LIFETIME: Duration = Duration::from_secs(600);

pub(super) async fn log_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let mut missing_reported = false;
    loop {
        let (path, selection) = {
            let settings = read(&state.settings);
            (settings.log_path(), settings.log_selection())
        };
        let Some(path) = path else {
            if !std::mem::replace(&mut missing_reported, true) {
                warn!("No EE.log found, looking again every 10 s");
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
            continue;
        };
        write(&state.status).log_file = Some(path.display().to_string());
        emit(&app, AppEvent::StatusUpdated(state.status_snapshot()));
        match wf_log::lines(&path, selection) {
            Ok(stream) => {
                replay_game_monitor(&app, &state, &path);
                write(&state.status).log_attached = true;
                emit(&app, AppEvent::StatusUpdated(state.status_snapshot()));
                pin_mut!(stream);
                while let Some(line) = stream.next().await {
                    match line {
                        Ok(source) => {
                            if let Some(event) = wf_log::classify(&source.line) {
                                let started = Instant::now();
                                if !matches!(event, LogEvent::InputMappingReset) {
                                    debug!(
                                        event = event.label(),
                                        origin = source.origin.label(),
                                        at = source.line.time,
                                        "Log event"
                                    );
                                }
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
                emit(&app, AppEvent::StatusUpdated(state.status_snapshot()));
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
            emit(&app, AppEvent::StatusUpdated(state.status_snapshot()));
            if found.is_some() {
                state.rescan.notify_one();
            } else {
                state.focus.forget();
            }
        }
        tokio::time::sleep(PROCESS_POLL).await;
    }
}

fn replay_game_monitor<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, path: &Path) {
    let lines = match wf_log::read_all(path) {
        Ok(lines) => lines,
        Err(error) => {
            warn!(%error, "Reading the existing EE.log for the game monitor failed");
            return;
        }
    };
    let monitor = lines
        .iter()
        .rev()
        .filter(|line| line.message.starts_with("Monitor Info ("))
        .find_map(|line| match wf_log::classify(line) {
            Some(event @ LogEvent::GameMonitor(_)) => Some((event, line.time)),
            _ => None,
        });
    if let Some((event, at)) = monitor {
        overlay::on_log_event(app, state, &event, at);
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
    if matches!(
        event,
        LogEvent::InventorySynced | LogEvent::InventoryCommitted
    ) {
        capture(app, state);
    }
    if matches!(event, LogEvent::RelicSelectScreenLoaded) {
        watch_relic_picker(state);
    }
    if let LogEvent::TradeScreen { visible } = event {
        state.trade_screen_open.store(visible, Ordering::SeqCst);
        if visible {
            let app = app.clone();
            let state = Arc::clone(state);
            tauri::async_runtime::spawn(async move { watch_trade(&app, &state).await });
        }
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
    let Some(screen) = scan_reward_screen(state).await else {
        return;
    };
    let owned = Arc::clone(state);
    let handled = blocking("handle reward screen", move || {
        lock(&owned.core).handle_reward_screen(Utc::now(), generation, screen)
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

fn watch_relic_picker(state: &Arc<AppState>) {
    if state.watching_picker.swap(true, Ordering::SeqCst) {
        return;
    }
    let state = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let mut closed_since = None;
        while started.elapsed() < PICKER_LIFETIME {
            match blocking("read relic picker", relic_picker).await {
                Some(Picker::Open(pick)) => {
                    closed_since = None;
                    if let Some(pick) = pick
                        && lock(&state.relic_picks)
                            .insert(pick.handle, pick.relic_type.clone())
                            .is_none()
                    {
                        debug!(relic_type = pick.relic_type, "Relic selected in the picker");
                    }
                }
                Some(Picker::Closed) | None => {
                    let since = *closed_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= PICKER_GRACE {
                        break;
                    }
                }
            }
            tokio::time::sleep(PICKER_TICK).await;
        }
        state.watching_picker.store(false, Ordering::SeqCst);
    });
}

enum Picker {
    Closed,
    Open(Option<wf_scan::RelicPick>),
}

fn scanned_trade(screen: TradeScreen) -> ScannedTrade {
    let items = |slots: Vec<wf_scan::TradeSlot>| {
        slots
            .into_iter()
            .map(|slot| ScannedTradeItem {
                name: slot.name,
                item_type: slot.item_type,
                count: i64::from(slot.count),
                fingerprint: slot.fingerprint,
            })
            .collect()
    };
    ScannedTrade {
        partner: screen.partner,
        offered: items(screen.offered),
        received: items(screen.received),
    }
}

fn completed_trade(state: &Arc<AppState>) -> anyhow::Result<Option<ScannedTrade>> {
    let reader = open_game().context("Attaching to the game process")?;
    let lua = LuaState::locate(reader.as_ref())?.context("The game's Lua state was not found")?;
    let started = Instant::now();
    loop {
        let open = state.trade_screen_open.load(Ordering::SeqCst);
        let completed = TradeScreen::read(reader.as_ref(), &lua).filter(|screen| screen.completed);
        if completed.is_some() || !open || started.elapsed() >= TRADE_WATCH_LIFETIME {
            return Ok(completed.map(scanned_trade));
        }
        std::thread::sleep(TRADE_TICK);
    }
}

fn count_trade<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let remaining = {
        let mut status = write(&state.status);
        let Some(left) = status.trades_remaining.filter(|left| *left > 0) else {
            return;
        };
        status.trades_remaining = Some(left - 1);
        left - 1
    };
    emit(app, AppEvent::StatusUpdated(state.status_snapshot()));
    if remaining == 0 {
        market_loop::last_trade_done(app, state);
    }
}

async fn watch_trade<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let owned = Arc::clone(state);
    let screen = match blocking("watch trade screen", move || completed_trade(&owned)).await {
        Some(Ok(Some(screen))) => screen,
        Some(Ok(None)) => {
            debug!("Trade screen closed without a completed trade");
            return;
        }
        None => return,
        Some(Err(error)) => {
            warn!(
                ?error,
                "Trade screen not watched, the trade is not recorded"
            );
            return;
        }
    };
    let owned = Arc::clone(state);
    let handled = blocking("record trade", move || {
        lock(&owned.core).handle_trade_screen(&screen, Utc::now())
    })
    .await;
    match handled {
        Some(Ok(events)) => {
            for event in &events {
                if let CoreEvent::TradeCompleted { trade, .. } = event {
                    info!(
                        offered = trade.offered.len(),
                        received = trade.received.len(),
                        plat = trade.plat,
                        "Trade completed"
                    );
                    count_trade(app, state);
                }
            }
            dispatch(app, state, events).await;
        }
        Some(Err(error)) => warn!(%error, "Core rejected the completed trade"),
        None => {}
    }
}

fn relic_picker() -> Picker {
    let open = open_game().ok().and_then(|reader| {
        let state = LuaState::locate(reader.as_ref()).ok().flatten()?;
        wf_scan::relic_picker_open(reader.as_ref(), &state)
            .then(|| wf_scan::relic_pick(reader.as_ref(), &state))
    });
    open.map_or(Picker::Closed, Picker::Open)
}

fn reward_screen(state: &AppState) -> anyhow::Result<ScannedRewards> {
    let reader = open_game().context("Attaching to the game process")?;
    let tiles = wf_scan::reward_screen(reader.as_ref()).context("Reading the reward screen")?;
    let own_relic_type = LuaState::locate(reader.as_ref())
        .context("Locating the Lua state")?
        .and_then(|state| wf_scan::confirmed_relic(reader.as_ref(), &state))
        .and_then(|handle| lock(&state.relic_picks).get(&handle).cloned());
    Ok(ScannedRewards {
        own_relic_type,
        rewards: tiles.into_iter().map(|tile| tile.store_item).collect(),
    })
}

async fn scan_reward_screen(state: &Arc<AppState>) -> Option<ScannedRewards> {
    let mut seen = ScannedRewards::default();
    for _ in 0..REWARD_TICKS {
        let owned = Arc::clone(state);
        let screen = match blocking("read reward screen", move || reward_screen(&owned)).await? {
            Ok(screen) => screen,
            Err(error) => {
                warn!(?error, "Reward screen read from game memory failed");
                return None;
            }
        };
        if !screen.rewards.is_empty() && screen == seen {
            if screen.own_relic_type.is_none() {
                debug!("Reward screen settled without a known relic for the local player");
            }
            return Some(screen);
        }
        seen = screen;
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
    emit(app, AppEvent::StatusUpdated(state.status_snapshot()));

    let owned = Arc::clone(state);
    let outcome = tauri::async_runtime::spawn_blocking(move || ingest(&owned))
        .await
        .map_err(anyhow::Error::from)
        .flatten();

    let ingested = match outcome {
        Ok(ingested) => ingested,
        Err(error) => {
            let cause = format!("{error:#}");
            warn!(cause, "Inventory not ingested");
            let mut status = write(&state.status);
            status.scanning = false;
            status.last_scan_error = Some(cause);
            drop(status);
            emit(app, AppEvent::StatusUpdated(state.status_snapshot()));
            return false;
        }
    };

    let (trades_remaining, snapshot) = {
        let core = lock(&state.core);
        (
            core.inventory().map(|inventory| inventory.trades_remaining),
            core.latest_snapshot().cloned(),
        )
    };
    let last_trade_done = {
        let mut status = write(&state.status);
        status.scanning = false;
        status.last_scan_at = Some(Utc::now());
        status.last_scan_error = None;
        status.source = InventorySource::Live;
        if let Some(snapshot) = &snapshot {
            status.remember_snapshot(snapshot);
        }
        let done =
            status.trades_remaining.is_some_and(|left| left > 0) && trades_remaining == Some(0);
        status.trades_remaining = trades_remaining;
        done
    };
    if last_trade_done {
        market_loop::last_trade_done(app, state);
    }

    emit(app, AppEvent::StatusUpdated(state.status_snapshot()));
    match ingested {
        Ingested::Newer { events } => {
            emit(app, AppEvent::InventoryUpdated(state.status_snapshot()));
            overlay::on_inventory_updated(app, state);
            state.prices_wake.notify_one();
            dispatch(app, state, events).await;
        }
        Ingested::Unchanged => {}
    }
    true
}

enum Ingested {
    Newer { events: Vec<CoreEvent> },
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
    ingest_buffer(state, &buffer)
}

fn ingest_buffer(state: &Arc<AppState>, buffer: &InventoryBuffer) -> anyhow::Result<Ingested> {
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
    Ok(Ingested::Newer { events })
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
