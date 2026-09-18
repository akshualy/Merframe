mod auctions;
pub mod commands;
pub mod envelope;
pub mod error;
mod focus;
pub mod images;
pub mod market;
pub mod overlay;
pub mod runtime;
pub mod settings;
pub mod state;

use std::path::Path;
use std::sync::Arc;

use tauri::plugin::TauriPlugin;
use tauri::{Manager, Runtime, Url};
use tracing::{debug, error, warn};
use tracing_subscriber::fmt::writer::MakeWriterExt;

use crate::state::{AppState, AppStateCell};

pub fn closes_app(label: &str) -> bool {
    label == "main"
}

pub fn is_own_page(url: &Url) -> bool {
    url.scheme() == "tauri"
        || url.host_str() == Some("tauri.localhost")
        || (cfg!(dev) && url.host_str() == Some("localhost"))
}

fn navigation_guard<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("navigation-guard")
        .on_navigation(|webview, url| {
            if is_own_page(url) {
                return true;
            }
            warn!(window = webview.label(), %url, "Navigation outside the app refused");
            false
        })
        .build()
}

fn prepare_data_dir(data_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        std::fs::set_permissions(data_dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn open_log(data_dir: &Path) -> std::io::Result<std::fs::File> {
    let path = data_dir.join("merframe.log");
    if std::fs::metadata(&path).is_ok_and(|log| log.len() > 4 * 1024 * 1024) {
        std::fs::rename(&path, path.with_extension("log.1"))?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;

        options.mode(0o600);
    }
    options.open(&path)
}

fn init_tracing(data_dir: &Path) -> std::io::Result<()> {
    prepare_data_dir(data_dir)?;
    let file = open_log(data_dir)?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("merframe_lib=debug,wf_=info")
            }),
        )
        .with_ansi(false)
        .with_writer(std::io::stderr.and(std::sync::Mutex::new(file)))
        .init();
    Ok(())
}

async fn publish_state(handle: tauri::AppHandle) {
    match AppState::build(&handle).await {
        Ok(state) => {
            let state = Arc::new(state);
            if handle
                .state::<AppStateCell>()
                .set(Arc::clone(&state))
                .is_err()
            {
                error!("App state built a second time");
            }
            debug!("App state published, commands live");
            overlay::apply_from(&handle, &state);
            runtime::spawn(handle.clone(), state);
            runtime::emit(&handle, "app-ready", ());
        }
        Err(error) => {
            let message = format!("{error:#}");
            error!(message, "Startup failed");
            runtime::emit(&handle, "app-error", message);
        }
    }
}

pub fn run() {
    let context = tauri::generate_context!();
    #[cfg(target_os = "linux")]
    let startup = overlay::adopt_xwayland();
    let builder = tauri::Builder::default()
        .plugin(navigation_guard())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::tabs::inventory_tab,
            commands::tabs::foundry_tab,
            commands::tabs::craft_tree,
            commands::tabs::mastery_tab,
            commands::tabs::resources_tab,
            commands::tabs::relic_planner_tab,
            commands::tabs::rivens_tab,
            commands::tabs::riven_comparables,
            commands::tabs::stats_tab,
            commands::tabs::toggle_favourite,
            commands::tabs::relics_for,
            commands::tabs::recommend,
            commands::app::game_status,
            commands::app::overlay_state,
            commands::app::overlay_page_ready,
            commands::app::rescan_inventory,
            commands::app::export,
            commands::app::settings_get,
            commands::app::settings_set,
            commands::app::test_notifications,
            commands::market::market_login,
            commands::market::market_logout,
            commands::market::market_my_orders,
            commands::market::market_post_order,
            commands::market::market_update_order,
            commands::market::market_close_order,
            commands::market::market_delete_order,
            commands::market::market_activity,
            commands::market::market_presence,
            commands::market::market_set_presence,
            commands::market::market_remove_all,
            commands::market::market_fix_orders,
            commands::market::market_set_visibility,
            commands::market::market_items,
            commands::market::market_item_orders,
            commands::market::market_post_riven,
            commands::market::market_my_auctions,
            commands::market::market_update_auction,
            commands::market::market_close_auction,
            commands::market::market_set_auctions_visibility,
            commands::world::worldstate,
            commands::app::refresh_prices,
            commands::app::open_url,
            commands::app::item_image,
            commands::app::prefetch_images,
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) && closes_app(window.label()) {
                window.app_handle().exit(0);
            }
        })
        .setup(move |app| {
            init_tracing(&state::data_dir(app.handle())?)?;
            #[cfg(target_os = "linux")]
            tracing::info!(
                session = ?startup.session,
                x_display = startup.x_display,
                gdk_backend_set = startup.gdk_backend_set,
                overlays_wanted = startup.overlays_wanted,
                xwayland = startup.adopted,
                "Overlay backend chosen"
            );
            app.manage(AppStateCell::new());
            tauri::async_runtime::spawn(publish_state(app.handle().clone()));
            Ok(())
        });

    if let Err(error) = builder.run(context) {
        error!(%error, "Tauri run failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::KINDS;

    #[test]
    fn main_window_closes_app() {
        assert!(closes_app("main"));
        for kind in KINDS {
            assert!(!closes_app(kind.label()));
        }
    }

    #[test]
    fn own_pages_only() {
        for own in ["tauri://localhost/index.html", "http://tauri.localhost/"] {
            assert!(is_own_page(&Url::parse(own).unwrap()), "{own}");
        }
        for foreign in [
            "https://warframe.market/im/chats",
            "http://tauri.localhost.example.com/",
            "file:///etc/passwd",
        ] {
            assert!(!is_own_page(&Url::parse(foreign).unwrap()), "{foreign}");
        }
    }

    #[test]
    fn log_rotation() {
        let dir = std::env::temp_dir().join("merframe-log-rotation");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        prepare_data_dir(&dir).unwrap();
        let path = dir.join("merframe.log");
        std::fs::write(&path, vec![b'x'; 5 * 1024 * 1024]).unwrap();

        drop(open_log(&dir).unwrap());
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
        assert_eq!(
            std::fs::metadata(dir.join("merframe.log.1")).unwrap().len(),
            5 * 1024 * 1024
        );

        drop(open_log(&dir).unwrap());
        assert!(!dir.join("merframe.log.2").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
