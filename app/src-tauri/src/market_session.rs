use std::sync::Arc;

use tauri::{AppHandle, Runtime};

use crate::market::MarketOrders;
use crate::runtime::{AppEvent, emit};
use crate::settings::{self, MarketAccount};
use crate::state::{AppState, write};

pub fn store_token<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    token: String,
) -> anyhow::Result<()> {
    let stored = settings::set_token(app, Some(&token));
    *write(&state.market) = Arc::new(state.anonymous_market().with_token(token));
    stored
}

pub fn record_account<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    account: MarketAccount,
) -> anyhow::Result<()> {
    let stored = settings::set_account(app, Some(&account));
    write(&state.status).market_account = Some(account);
    emit(app, AppEvent::StatusUpdated(state.status_snapshot()));
    stored
}

pub fn clear<R: Runtime>(app: &AppHandle<R>, state: &AppState) -> anyhow::Result<()> {
    let cleared = settings::set_token(app, None).and(settings::set_account(app, None));
    *write(&state.market) = Arc::new(state.anonymous_market());

    let mut status = write(&state.status);
    status.market_account = None;
    status.market_unread = 0;
    drop(status);

    state
        .listings
        .remember(&state.core, Some(&MarketOrders::default()), Some(&[]));
    emit(app, AppEvent::StatusUpdated(state.status_snapshot()));
    cleared
}
