use std::sync::Arc;

use anyhow::Context;
use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};
use tauri_plugin_opener::OpenerExt;

use super::{Shared, ask_path, game_log_path, pick_log, ready};
use crate::error::{CommandError, CommandResult};
use crate::event_feed::EventFeed;
use crate::overlay;
use crate::runtime::{self, AppEvent};
use crate::settings::{self, Settings};
use crate::state::{GameStatus, LOG_FILE, lock, read};

#[tauri::command]
pub async fn game_status(state: Shared<'_>) -> CommandResult<GameStatus> {
    let state = ready(&state).await?;
    Ok(state.status_snapshot())
}

#[tauri::command]
pub fn updates_supported() -> bool {
    tauri::utils::platform::bundle_type().is_some()
}

#[tauri::command]
pub async fn overlay_state(state: Shared<'_>) -> CommandResult<overlay::OverlayState> {
    Ok(ready(&state).await?.overlays.snapshot())
}

#[tauri::command]
pub async fn overlay_page_ready<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    state: Shared<'_>,
) -> CommandResult<()> {
    overlay::on_page_ready(window.app_handle(), &ready(&state).await?, window.label());
    Ok(())
}

#[tauri::command]
pub async fn overlay_notify<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    title: String,
    body: String,
) -> CommandResult<()> {
    overlay::notify(&app, &ready(&state).await?, title, body);
    Ok(())
}

#[tauri::command]
pub async fn overlay_test<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    kind: overlay::Kind,
) -> CommandResult<()> {
    overlay::toggle_preview(&app, &ready(&state).await?, kind)?;
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
    let state = ready(&state).await?;
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
    let state = ready(&state).await?;
    let dialog = app.dialog().file().set_title("Export inventory JSON");
    let Some(dir) = ask_path(dialog, FileDialogBuilder::blocking_pick_folder).await? else {
        return Ok(None);
    };

    let target = dir.clone();
    tauri::async_runtime::spawn_blocking(move || lock(&state.core).export(&target, Utc::now()))
        .await??;
    Ok(Some(dir.display().to_string()))
}

#[tauri::command]
pub async fn pick_log_file<R: Runtime>(app: AppHandle<R>) -> CommandResult<Option<String>> {
    let picked = pick_log(&app, "Choose EE.log").await?;
    Ok(picked.map(|path| path.display().to_string()))
}

#[tauri::command]
pub async fn settings_get(state: Shared<'_>) -> CommandResult<Settings> {
    let state = ready(&state).await?;
    Ok(read(&state.settings).clone())
}

#[tauri::command]
pub async fn settings_set<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    settings: Settings,
) -> CommandResult<Settings> {
    let state = ready(&state).await?;
    settings.event_feed.check()?;
    let previous = settings::apply(&app, &state, &settings)?;
    if previous.alerts != settings.alerts {
        runtime::emit(&app, AppEvent::WorldStateUpdated);
    }
    overlay::apply_from(&app, &state);
    app.state::<EventFeed>().configure(settings.event_feed);
    Ok(settings)
}

#[tauri::command]
pub async fn test_notifications<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    settings: Settings,
) -> CommandResult<()> {
    let state = ready(&state).await?;
    runtime::test_notifications(&app, &state, &settings).await?;
    Ok(())
}

#[tauri::command]
pub async fn app_log_file(state: Shared<'_>) -> CommandResult<String> {
    let state = ready(&state).await?;
    Ok(state.data_dir.join(LOG_FILE).display().to_string())
}

#[tauri::command]
pub async fn open_data_folder<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
) -> CommandResult<()> {
    let state = ready(&state).await?;
    Ok(app
        .opener()
        .reveal_item_in_dir(state.data_dir.join(LOG_FILE))
        .context("Opening the data folder")?)
}

#[tauri::command]
pub async fn open_game_log_folder<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
) -> CommandResult<()> {
    let state = ready(&state).await?;
    Ok(app
        .opener()
        .reveal_item_in_dir(game_log_path(&state)?)
        .context("Opening the EE.log folder")?)
}

#[tauri::command]
pub async fn item_image(state: Shared<'_>, image_name: String) -> CommandResult<String> {
    let state = ready(&state).await?;
    state.images.get(&image_name).await
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ImagePath {
    pub name: String,
    pub path: String,
}

#[tauri::command]
pub async fn prefetch_images(
    state: Shared<'_>,
    names: Vec<String>,
) -> CommandResult<Vec<ImagePath>> {
    let state = ready(&state).await?;
    let mut wanted = names;
    wanted.sort_unstable();
    wanted.dedup();
    let requests = wanted
        .into_iter()
        .map(|name| {
            let shared = Arc::clone(&state);
            async move {
                let path = shared.images.get(&name).await.ok()?;
                Some(ImagePath { name, path })
            }
        })
        .collect::<Vec<_>>();
    let held = futures_util::future::join_all(requests)
        .await
        .into_iter()
        .flatten()
        .collect();
    Ok(held)
}

#[cfg(test)]
mod tests {
    fn browser_patterns() -> Vec<glob::Pattern> {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../../capabilities/default.json")).unwrap();
        let permissions = capability["permissions"].as_array().unwrap();
        let opener = permissions
            .iter()
            .filter(|permission| {
                permission
                    .as_str()
                    .or_else(|| permission["identifier"].as_str())
                    .is_some_and(|id| id.starts_with("opener:"))
            })
            .collect::<Vec<_>>();
        assert_eq!(opener.len(), 1, "Only scoped URL opening should be granted");
        assert_eq!(opener[0]["identifier"], "opener:allow-open-url");
        // Use the same glob matcher as the opener plugin against the real capability.
        opener[0]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| glob::Pattern::new(entry["url"].as_str().unwrap()).unwrap())
            .collect()
    }

    #[test]
    fn browser_links_allow_web_pages() {
        let patterns = browser_patterns();
        for url in [
            "https://wiki.warframe.com",
            "https://wiki.warframe.com/w/Braton_Prime",
            "https://warframe.market",
            "https://warframe.market/items/braton_prime_set",
            "https://warframe.market/im/chats",
            "https://warframe.market/auctions/search?type=riven&weapon_url_name=kuva_shildeg",
            "https://yareli.net/merframe",
            "https://github.com/akshualy/Merframe",
            "https://github.com/WFCD/warframe-items",
            "https://alecaframe.com",
            "https://browse.wf/arbys",
            "https://browse.wf/about",
        ] {
            assert!(patterns.iter().any(|pattern| pattern.matches(url)), "{url}");
        }
    }

    #[test]
    fn browser_links_reject_untrusted_destinations() {
        let patterns = browser_patterns();
        for url in [
            "file:///C:/Windows/System32/calc.exe",
            "C:\\Windows\\System32\\calc.exe",
            "\\\\server\\share\\program.exe",
            "javascript:alert(1)",
            "ms-settings:windowsupdate",
            "steam://run/123",
            "mailto:user@example.com",
            "tel:12345",
            "http://wiki.warframe.com/w/Braton_Prime",
            "https://example.com/",
            "https://wiki.warframe.com.evil.example/",
            "https://wiki.warframe.com@evil.example/",
            "https://user:password@wiki.warframe.com/",
            "https://wiki.warframe.com:9999/",
            "https://warframe.market.evil.example/",
            "https://warframe.market@evil.example/",
            "https://user@warframe.market/",
            "https://",
            "not a URL",
        ] {
            assert!(
                !patterns.iter().any(|pattern| pattern.matches(url)),
                "{url}"
            );
        }
    }
}
