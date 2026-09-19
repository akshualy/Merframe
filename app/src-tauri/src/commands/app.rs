use std::sync::Arc;

use anyhow::Context;
use chrono::Utc;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_opener::OpenerExt;

use super::{Shared, ready};
use crate::error::{CommandError, CommandResult};
use crate::overlay;
use crate::runtime;
use crate::settings::{self, Settings};
use crate::state::{GameStatus, lock, read, write};

#[tauri::command]
pub async fn game_status(state: Shared<'_>) -> CommandResult<GameStatus> {
    let state = ready(&state)?;
    Ok(state.status_snapshot())
}

#[tauri::command]
pub async fn overlay_state(state: Shared<'_>) -> CommandResult<overlay::OverlayState> {
    Ok(ready(&state)?.overlays.snapshot())
}

#[tauri::command]
pub async fn overlay_page_ready<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    state: Shared<'_>,
) -> CommandResult<()> {
    overlay::on_page_ready(window.app_handle(), &ready(&state)?, window.label());
    Ok(())
}

#[tauri::command]
pub async fn overlay_content_shrank<R: Runtime>(window: tauri::WebviewWindow<R>) {
    overlay::on_content_shrank(&window);
}

#[tauri::command]
pub async fn rescan_inventory<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
) -> CommandResult<GameStatus> {
    let state = ready(&state)?;
    if runtime::acquire(&app, &state).await {
        Ok(state.status_snapshot())
    } else {
        let status = state.status_snapshot();
        Err(CommandError::from(
            status
                .last_scan_error
                .unwrap_or_else(|| "The inventory scan failed".to_owned()),
        ))
    }
}

#[tauri::command]
pub async fn export<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
) -> CommandResult<Option<String>> {
    let state = ready(&state)?;
    let picked = app
        .dialog()
        .file()
        .set_title("Export inventory JSON")
        .blocking_pick_folder();
    let Some(dir) = picked.as_ref().and_then(FilePath::as_path) else {
        return Ok(None);
    };
    lock(&state.core).export(dir, Utc::now())?;
    Ok(Some(dir.display().to_string()))
}

#[tauri::command]
pub async fn settings_get(state: Shared<'_>) -> CommandResult<Settings> {
    let state = ready(&state)?;
    Ok(read(&state.settings).clone())
}

#[tauri::command]
pub async fn settings_set<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    settings: Settings,
) -> CommandResult<Settings> {
    let state = ready(&state)?;
    settings::save(&app, &settings)?;
    lock(&state.core).set_alert_settings(settings.alerts.clone());
    *write(&state.settings) = settings.clone();
    overlay::apply_from(&app, &state);
    Ok(settings)
}

#[tauri::command]
pub async fn test_notifications<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    settings: Settings,
) -> CommandResult<()> {
    let state = ready(&state)?;
    runtime::test_notifications(&app, &state, &settings).await?;
    Ok(())
}

#[tauri::command]
pub async fn refresh_prices<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
) -> CommandResult<usize> {
    let state = ready(&state)?;
    Ok(runtime::refresh_prices(&app, &state).await?)
}

#[tauri::command]
pub async fn open_url<R: Runtime>(app: AppHandle<R>, url: String) -> CommandResult<()> {
    Ok(app
        .opener()
        .open_url(url, None::<&str>)
        .context("Opening the URL")?)
}

#[tauri::command]
pub async fn open_data_folder<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
) -> CommandResult<()> {
    let state = ready(&state)?;
    Ok(app
        .opener()
        .reveal_item_in_dir(state.data_dir.join("merframe.log"))
        .context("Opening the data folder")?)
}

#[tauri::command]
pub async fn open_game_log_folder<R: Runtime>(app: AppHandle<R>) -> CommandResult<()> {
    let log = wf_log::default_log_path().context("EE.log not found on this machine")?;
    Ok(app
        .opener()
        .reveal_item_in_dir(log)
        .context("Opening the EE.log folder")?)
}

#[tauri::command]
pub async fn item_image(state: Shared<'_>, image_name: String) -> CommandResult<String> {
    let state = ready(&state)?;
    state.images.get(&image_name).await
}

#[tauri::command]
pub async fn prefetch_images(state: Shared<'_>, names: Vec<String>) -> CommandResult<usize> {
    let state = ready(&state)?;
    let mut wanted = names;
    wanted.sort_unstable();
    wanted.dedup();
    let requests = wanted
        .into_iter()
        .map(|name| {
            let shared = Arc::clone(&state);
            async move { shared.images.get(&name).await.is_ok() }
        })
        .collect::<Vec<_>>();
    let cached = futures_util::future::join_all(requests)
        .await
        .into_iter()
        .filter(|outcome| *outcome)
        .count();
    Ok(cached)
}
