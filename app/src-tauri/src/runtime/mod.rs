use std::sync::Arc;

use anyhow::Context;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;
use tracing::{debug, error, info, warn};
use wf_core::CoreEvent;

use crate::envelope::CoreEventEnvelope;
use crate::market::{MarketSnapshot, Presence};
use crate::notice::Notice;
use crate::overlay::{self, OverlayState};
use crate::settings::Settings;
use crate::state::{AppState, GameStatus, InventorySource, lock, read, write};

mod feeds;
mod game;
mod market_loop;

use feeds::{price_task, world_state_task};
use game::{ducatering_task, inventory_task, kiosk_task, log_task, process_task, trade_task};
use market_loop::{auto_close, market_presence_task, market_task};

pub use game::acquire;
pub use market_loop::MarketAutoClose;
pub(crate) use market_loop::trade_side;

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
pub enum AppEvent {
    CoreEvent(CoreEventEnvelope),
    StatusUpdated(GameStatus),
    InventoryUpdated(GameStatus),
    WorldStateUpdated,
    RivenDataUpdated,
    PricesUpdated,
    FavouriteUpdated,
    MarketUpdated(MarketSnapshot),
    MarketAutoClosed(MarketAutoClose),
    MarketSignedOut,
    MarketPresence(Presence),
    OverlayState(Box<OverlayState>),
}

const RIVEN_DATA_KEY: &str = "riven_data";
const APP_TITLE: &str = "Merframe";
const NOTIFICATION_SOUND: &str = if cfg!(windows) {
    "Default"
} else {
    "message-new-instant"
};

pub fn spawn<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    tauri::async_runtime::spawn(bootstrap(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(process_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(log_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(inventory_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(ducatering_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(trade_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(kiosk_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(world_state_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(market_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(market_presence_task(app.clone(), Arc::clone(&state)));
    tauri::async_runtime::spawn(price_task(app, state));
}

async fn bootstrap<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    load_cached_inventory(&app, &state).await;
    let riven_types = lock(&state.core).catalog().data().riven_data().len();
    if riven_types == 0 {
        warn!("Game data has no riven stat tables");
    } else {
        info!(riven_types, "Riven stat tables loaded");
    }
    emit(&app, AppEvent::StatusUpdated(state.status_snapshot()));
}

async fn load_cached_inventory<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let owned = Arc::clone(state);
    let loaded = tauri::async_runtime::spawn_blocking(move || {
        lock(&owned.core)
            .load_snapshot()
            .context("Loading the stored inventory")
    })
    .await
    .map_err(anyhow::Error::from)
    .flatten();
    match loaded {
        Ok(Some(snapshot)) => {
            let mut status = write(&state.status);
            status.source = InventorySource::Cached;
            status.remember_snapshot(&snapshot);
            status.trades_remaining = lock(&state.core)
                .inventory()
                .map(|inventory| inventory.trades_remaining);
            drop(status);
            info!(
                snapshot = snapshot.id.0,
                last_sync = snapshot.last_sync_oid,
                taken_at = snapshot.taken_at.to_rfc3339(),
                "Last inventory snapshot restored from the store"
            );
            emit(app, AppEvent::StatusUpdated(state.status_snapshot()));
            emit(app, AppEvent::InventoryUpdated(state.status_snapshot()));
            state.prices_wake.notify_one();
        }
        Ok(None) => debug!("No stored inventory snapshot yet"),
        Err(error) => warn!(?error, "Stored inventory unreadable"),
    }
}

pub async fn dispatch<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    events: Vec<CoreEvent>,
) {
    for event in events {
        debug!(event = event.label(), "Core event");
        emit(
            app,
            AppEvent::CoreEvent(CoreEventEnvelope::wrap(event.clone())),
        );
        overlay::on_core_event(app, state, &event);
        deliver(app, state, &event).await;
    }
}

async fn deliver<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, event: &CoreEvent) {
    let settings = read(&state.settings).clone();
    match event {
        CoreEvent::NewConversation { player, .. } => {
            if state
                .focus
                .suppresses(settings.notifications.notification_only_background)
            {
                return;
            }
            let notice = Notice::conversation(player);
            if settings.notifications.windows_notifications_enabled
                && let Err(error) = show(app, &notice.title, &notice.body, &settings).await
            {
                warn!(?error, player, "Conversation notification not shown");
            }
            if settings.discord.discord_notifications_enabled {
                let message = settings
                    .discord
                    .discord_message_template
                    .replace("{tenno}", player);
                post_webhook(state, settings.discord.discord_webhook.as_deref(), message).await;
            }
        }
        CoreEvent::RelicRewardScreen { rewards, .. } => {
            if settings.copy_relic_rewards {
                let line = lock(&state.core).recommend(rewards).chat_line();
                if !line.is_empty()
                    && let Err(error) = app.clipboard().write_text(line)
                {
                    warn!(%error, "Relic rewards not copied to the clipboard");
                }
            }
        }
        CoreEvent::TradeCompleted { trade, .. } => {
            if settings.market.market_auto_close {
                auto_close(app, state, trade).await;
            }
        }
        CoreEvent::FissureAlert { fissure } => {
            let mirrored = settings.discord.discord_fissure_alerts;
            alert(app, state, &settings, &Notice::fissure(fissure), mirrored).await;
        }
        CoreEvent::TimerAlert {
            name,
            next_state,
            remaining_secs,
            ..
        } => {
            let mirrored = settings.discord.discord_timer_alerts;
            let notice = Notice::timer(name, next_state, *remaining_secs);
            alert(app, state, &settings, &notice, mirrored).await;
        }
        CoreEvent::InventoryUpdated(_) => {}
    }
}

async fn alert<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    settings: &Settings,
    notice: &Notice,
    mirrored: bool,
) {
    if settings.notifications.windows_notifications_enabled
        && let Err(error) = show(app, &notice.title, &notice.body, settings).await
    {
        warn!(
            ?error,
            notice.title, notice.body, "Game event notification not shown"
        );
    }
    if mirrored {
        let webhook = settings.discord.discord_webhook.as_deref();
        post_webhook(state, webhook, notice.webhook_message()).await;
    }
}

#[cfg(not(target_os = "linux"))]
fn show<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: &str,
    settings: &Settings,
) -> std::future::Ready<anyhow::Result<()>> {
    let mut builder = app.notification().builder().title(title).body(body);
    if settings.notifications.sound_notifications_enabled {
        builder = builder.sound(NOTIFICATION_SOUND);
    }
    std::future::ready(builder.show().context("Sending the desktop notification"))
}

#[cfg(target_os = "linux")]
async fn show<R: Runtime>(
    _app: &AppHandle<R>,
    title: &str,
    body: &str,
    settings: &Settings,
) -> anyhow::Result<()> {
    let mut notification = notify_rust::Notification::new();
    notification.summary(title).body(body).auto_icon();
    if settings.notifications.sound_notifications_enabled {
        notification.sound_name(NOTIFICATION_SOUND);
    }
    let handle = notification
        .show_async()
        .await
        .context("Sending the desktop notification")?;
    tauri::async_runtime::spawn(async move {
        handle.wait_for_action_async(|_| {}).await;
    });
    Ok(())
}

pub async fn test_notifications<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    settings: &Settings,
) -> anyhow::Result<()> {
    let discord = &settings.discord;
    let webhook = if discord.discord_notifications_enabled
        || discord.discord_fissure_alerts
        || discord.discord_timer_alerts
    {
        Some(
            settings
                .discord
                .discord_webhook
                .as_deref()
                .filter(|url| !url.is_empty())
                .context("The Discord webhook URL is empty")?,
        )
    } else {
        None
    };
    if !settings.notifications.windows_notifications_enabled && webhook.is_none() {
        anyhow::bail!("No notification channels are enabled");
    }
    if settings.notifications.windows_notifications_enabled {
        show(app, APP_TITLE, "Notifications are working", settings).await?;
    }
    if let Some(url) = webhook {
        send_webhook(state, url, "Merframe notifications are working").await?;
    }
    Ok(())
}

async fn post_webhook(state: &Arc<AppState>, url: Option<&str>, message: String) {
    let Some(url) = url.filter(|url| !url.is_empty()) else {
        return;
    };
    if let Err(error) = send_webhook(state, url, &message).await {
        warn!(?error, "Discord webhook post failed");
    }
}

async fn send_webhook(state: &Arc<AppState>, url: &str, message: &str) -> anyhow::Result<()> {
    let body = serde_json::json!({ "content": message });
    state
        .http
        .post(url)
        .json(&body)
        .send()
        .await
        .context("Sending the Discord webhook")?
        .error_for_status()
        .context("The Discord webhook returned an error")?;
    Ok(())
}

pub fn emit<R: Runtime>(app: &AppHandle<R>, event: AppEvent) {
    if let Err(error) = app.emit("app-event", event) {
        warn!(%error, "Webview emit failed");
    }
}

pub(crate) async fn blocking<T, F>(task: &'static str, job: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(job).await {
        Ok(value) => Some(value),
        Err(error) => {
            error!(task, %error, "Blocking task panicked");
            None
        }
    }
}
