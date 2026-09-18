use std::sync::Arc;

use anyhow::Context;
use tauri::{AppHandle, Emitter, Runtime};
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;
use tracing::{debug, error, info, warn};
use wf_core::CoreEvent;
use wf_market::RivenData;

use crate::envelope::CoreEventEnvelope;
use crate::overlay;
use crate::settings::Settings;
use crate::state::{AppState, InventorySource, lock, read, write};

mod feeds;
mod game;
mod market_loop;

use feeds::{price_task, world_state_task};
use game::{inventory_task, log_task, process_task};
use market_loop::{auto_close, market_presence_task, market_task};

pub use feeds::refresh_prices;
pub use game::acquire;
pub use market_loop::{MarketAutoClose, MarketSnapshot};

pub const INVENTORY_UPDATED: &str = "inventory-updated";
pub const STATUS_UPDATED: &str = "status-updated";
pub const MARKET_AUTO_CLOSED: &str = "market-auto-closed";

const RIVEN_DATA_KEY: &str = "riven_data";
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
    match state.market().riven_attributes().await {
        Ok(attributes) => lock(&state.core).set_riven_attributes(attributes),
        Err(error) => {
            warn!(error = %error.brief(), "Riven attribute list request to warframe.market failed");
        }
    }
    load_cached_riven_data(&state).await;
    emit(&app, STATUS_UPDATED, state.status_snapshot());
}

async fn load_cached_riven_data(state: &Arc<AppState>) {
    let owned = Arc::clone(state);
    let loaded = tauri::async_runtime::spawn_blocking(move || {
        lock(&owned.core)
            .store()
            .setting::<RivenData>(RIVEN_DATA_KEY)
            .context("Reading the stored riven data")
    })
    .await
    .map_err(anyhow::Error::from)
    .flatten();
    match loaded {
        Ok(Some(data)) => {
            info!(
                weapons = data.weapons.len(),
                good_rolls = data.good_rolls.len(),
                updated_at = data.updated_at,
                "Riven weapon and roll data restored from the store"
            );
            lock(&state.core).set_riven_data(data);
        }
        Ok(None) => debug!("No stored riven data yet"),
        Err(error) => warn!(?error, "Stored riven data unreadable"),
    }
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
            status.last_sync_oid = Some(snapshot.last_sync_oid.clone());
            status.last_sync_at = Some(snapshot.taken_at);
            drop(status);
            info!(
                snapshot = snapshot.id.0,
                last_sync = snapshot.last_sync_oid,
                taken_at = snapshot.taken_at.to_rfc3339(),
                "Last inventory snapshot restored from the store"
            );
            emit(app, STATUS_UPDATED, state.status_snapshot());
            emit(app, INVENTORY_UPDATED, state.status_snapshot());
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
        emit(app, "core-event", CoreEventEnvelope::wrap(event.clone()));
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
            if settings.notifications.windows_notifications_enabled
                && let Err(error) = show(app, &conversation_text(player), &settings).await
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
        CoreEvent::TradeCompleted { trade, .. } => {
            if settings.market.market_auto_close {
                auto_close(app, state, trade).await;
            }
        }
        _ => {
            if settings.notifications.windows_notifications_enabled
                && let Some(body) = notification_text(event)
                && let Err(error) = show(app, &body, &settings).await
            {
                warn!(?error, body, "Game event notification not shown");
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn show<R: Runtime>(
    app: &AppHandle<R>,
    body: &str,
    settings: &Settings,
) -> std::future::Ready<anyhow::Result<()>> {
    let mut builder = app.notification().builder().title("Merframe").body(body);
    if settings.notifications.sound_notifications_enabled {
        builder = builder.sound(NOTIFICATION_SOUND);
    }
    std::future::ready(builder.show().context("Sending the desktop notification"))
}

#[cfg(target_os = "linux")]
async fn show<R: Runtime>(
    _app: &AppHandle<R>,
    body: &str,
    settings: &Settings,
) -> anyhow::Result<()> {
    let mut notification = notify_rust::Notification::new();
    notification.summary("Merframe").body(body).auto_icon();
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
    let webhook = if settings.discord.discord_notifications_enabled {
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
        show(app, "Notifications are working", settings).await?;
    }
    if let Some(url) = webhook {
        send_webhook(state, url, "Merframe notifications are working").await?;
    }
    Ok(())
}

pub fn conversation_text(player: &str) -> String {
    format!("You have a new in-game conversation from {player}")
}

pub fn fissure_text(fissure: &wf_core::FissureInfo) -> String {
    let location = fissure
        .planet
        .or(fissure.node_name)
        .unwrap_or(fissure.node_id.as_str());
    let steel_path = if fissure.steel_path {
        " [Steel Path]"
    } else {
        ""
    };
    format!(
        "{} on {} - {} fissure ({} min left){}",
        fissure.mission_name,
        location,
        fissure.tier,
        fissure.remaining_secs / 60,
        steel_path
    )
}

fn notification_text(event: &CoreEvent) -> Option<String> {
    match event {
        CoreEvent::FissureAlert { fissure } => Some(fissure_text(fissure)),
        CoreEvent::TimerAlert {
            name,
            next_state,
            remaining_secs,
            ..
        } => Some(format!(
            "{name} turns {next_state} in {} minutes",
            remaining_secs / 60
        )),
        _ => None,
    }
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

pub fn emit<R: Runtime, T: serde::Serialize + Clone>(app: &AppHandle<R>, event: &str, payload: T) {
    if let Err(error) = app.emit(event, payload) {
        warn!(event, %error, "Webview emit failed");
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
