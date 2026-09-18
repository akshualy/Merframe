use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::Utc;
use tauri::{AppHandle, Runtime};
use tracing::{debug, info, warn};
use wf_core::CoreEvent;
use wf_market::{Etagged, PriceTable, RivenData, bulk_prices, bulk_riven_data};
use wf_worldstate::WorldState;

use super::{RIVEN_DATA_KEY, STATUS_UPDATED, dispatch, emit};
use crate::state::{AppState, lock, read, write};

const PRICE_RETRY: Duration = Duration::from_secs(60);

pub(super) async fn world_state_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let mut fetched_at: Option<Instant> = None;
    loop {
        let interval = read(&state.settings).world_state_interval();
        if fetched_at.is_none_or(|at| at.elapsed() >= interval) {
            fetched_at = Some(Instant::now());
            match refresh_world_state(&app, &state).await {
                Ok(events) => dispatch(&app, &state, events).await,
                Err(error) => warn!(?error, "World state refresh failed"),
            }
        } else {
            let events = lock(&state.core).evaluate_world_state(Utc::now());
            dispatch(&app, &state, events).await;
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

async fn refresh_world_state<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
) -> anyhow::Result<Vec<CoreEvent>> {
    let body = wf_worldstate::fetch_body(&state.http)
        .await
        .context("Requesting the world state")?;
    let now = Utc::now();
    let parsed = WorldState::parse(&body).context("Parsing the world state")?;
    *write(&state.world) = Some(parsed);
    write(&state.status).world_state_at = Some(now);
    let events = {
        let mut core = lock(&state.core);
        core.ingest_world_state(&body, now)
            .context("Ingesting the world state")?
    };
    emit(app, "worldstate-updated", state.status_snapshot());
    Ok(events)
}

pub(super) async fn price_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let mut client = bulk_prices(state.http.clone());
    let mut rivens = bulk_riven_data(state.http.clone());
    let mut checked_at: Option<Instant> = None;
    let mut rivens_checked_at: Option<Instant> = None;
    loop {
        let interval = read(&state.settings).price_interval();
        let mut wait = interval;
        if checked_at.is_none_or(|at| at.elapsed() >= interval) {
            match load_price_table(&app, &state, &mut client).await {
                Ok(_) => checked_at = Some(Instant::now()),
                Err(error) => {
                    checked_at = None;
                    wait = PRICE_RETRY;
                    warn!(
                        url = client.url(),
                        error = %error.brief(),
                        "Price table not fetched, next attempt in a minute"
                    );
                }
            }
        }
        if rivens_checked_at.is_none_or(|at| at.elapsed() >= Duration::from_hours(24)) {
            match load_riven_data(&app, &state, &mut rivens).await {
                Ok(()) => rivens_checked_at = Some(Instant::now()),
                Err(error) => {
                    rivens_checked_at = None;
                    wait = wait.min(PRICE_RETRY);
                    warn!(
                        url = rivens.url(),
                        error = %error.brief(),
                        "Riven weapon and roll data not fetched, next attempt in a minute"
                    );
                }
            }
        }
        tokio::select! {
            () = state.prices_wake.notified() => {}
            () = tokio::time::sleep(wait) => {}
        }
    }
}

async fn load_riven_data<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    client: &mut Etagged<RivenData>,
) -> wf_market::Result<()> {
    let Some(data) = client.fetch().await? else {
        debug!("Riven data etag matches, nothing to load");
        return Ok(());
    };
    info!(
        weapons = data.weapons.len(),
        good_rolls = data.good_rolls.len(),
        updated_at = data.updated_at,
        "Riven weapon and roll data downloaded"
    );
    let mut core = lock(&state.core);
    if let Err(error) = core.store().set_setting(RIVEN_DATA_KEY, &data) {
        warn!(%error, "Riven data store write failed");
    }
    core.set_riven_data(data);
    drop(core);
    emit(app, "riven-data-updated", state.status_snapshot());
    Ok(())
}

pub async fn refresh_prices<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
) -> wf_market::Result<usize> {
    let mut client = bulk_prices(state.http.clone());
    load_price_table(app, state, &mut client).await
}

async fn load_price_table<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    client: &mut Etagged<PriceTable>,
) -> wf_market::Result<usize> {
    let now = Utc::now();
    let Some(table) = client.fetch().await? else {
        state.prices.checked(now);
        debug!("Bulk price table not modified since the last fetch");
        emit(app, STATUS_UPDATED, state.status_snapshot());
        return Ok(0);
    };
    let loaded = state.prices.load(&table, now);
    info!(
        loaded,
        count = table.count,
        updated_at = table.updated_at,
        "Prices refreshed from the bulk table"
    );
    emit(app, "prices-updated", loaded);
    emit(app, STATUS_UPDATED, state.status_snapshot());
    Ok(loaded)
}
