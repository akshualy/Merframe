use std::sync::Arc;

use tauri::{AppHandle, Runtime};
use tracing::{info, warn};

use super::{market_refresh, market_snapshot, signed_in_slug};
use crate::runtime::{AppEvent, emit};
use crate::state::{AppState, read, write};

pub(crate) fn last_trade_done<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let settings = read(&state.settings).market.last_trade;
    if settings.market_hide_after_last_trade {
        let app = app.clone();
        let state = Arc::clone(state);
        let auctions = settings.market_hide_auctions_after_last_trade;
        tauri::async_runtime::spawn(async move { hide_listings(&app, &state, auctions).await });
    }

    if settings.market_offline_after_last_trade {
        offline_after_last_trade(app, state);
    }
}

async fn hide_listings<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, auctions: bool) {
    let Some(slug) = signed_in_slug(state) else {
        return;
    };

    let client = state.market();
    match client.set_all_orders_visibility(false).await {
        Ok(update) => info!(
            orders = update.updated,
            "Last trade of the day completed, orders hidden"
        ),
        Err(error) => warn!(error = %error.brief(), "Hiding orders after the last trade failed"),
    }

    if auctions {
        match client.set_auctions_visibility(false).await {
            Ok(()) => info!("Last trade of the day completed, auctions hidden"),
            Err(error) => {
                warn!(error = %error.brief(), "Hiding auctions after the last trade failed");
            }
        }
    }

    let refresh = market_refresh(state, &slug).await;
    if !refresh.is_empty() {
        let snapshot = market_snapshot(state, refresh).await;
        emit(app, AppEvent::MarketUpdated(snapshot));
    }
}

fn offline_after_last_trade<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let mut guard = write(&state.market_presence);
    if !guard.is_live_online() {
        return;
    }

    *guard = guard.taken_offline();
    let presence = *guard;
    drop(guard);
    state.market_presence_wake.notify_one();

    info!("Last trade of the day completed, warframe.market status set to offline");
    emit(app, AppEvent::MarketPresence(presence));
}
