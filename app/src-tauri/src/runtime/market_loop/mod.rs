mod auto_close;
mod last_trade;
mod poll;
mod presence;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Runtime};
use tracing::warn;
use wf_market::{Auction, Order};

use crate::market::{self, MarketSnapshot};
use crate::market_session;
use crate::runtime::{AppEvent, emit};
use crate::state::{AppState, read};

pub use auto_close::MarketAutoClose;
pub(super) use auto_close::auto_close;
pub(crate) use auto_close::trade_side;
pub(super) use last_trade::last_trade_done;
pub(super) use poll::market_task;
pub(super) use presence::market_presence_task;

const MARKET_SIGNED_OUT_POLL: Duration = Duration::from_secs(30);

fn signed_in_slug(state: &Arc<AppState>) -> Option<String> {
    read(&state.status)
        .market_account
        .as_ref()
        .map(|account| account.slug.clone())
}

fn sign_out<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    if let Err(error) = market_session::clear(app, state) {
        warn!(%error, "Clearing the stored market session failed");
    }
    emit(app, AppEvent::MarketSessionRejected);
}

struct MarketRefresh {
    orders: Option<Vec<Order>>,
    auctions: Option<Vec<Auction>>,
}

impl MarketRefresh {
    fn is_empty(&self) -> bool {
        self.orders.is_none() && self.auctions.is_none()
    }

    fn is_complete(&self) -> bool {
        self.orders.is_some() && self.auctions.is_some()
    }
}

fn refresh_failed(kind: &'static str, error: &wf_market::MarketError) {
    warn!(kind, error = %error.brief(), "Warframe.market refresh failed");
}

async fn market_refresh(state: &Arc<AppState>, slug: &str) -> MarketRefresh {
    let client = state.market();
    let orders = client
        .orders_my()
        .await
        .inspect_err(|error| refresh_failed("orders", error))
        .ok();
    let auctions = client
        .auctions_my(slug)
        .await
        .inspect_err(|error| refresh_failed("auctions", error))
        .ok();
    MarketRefresh { orders, auctions }
}

async fn market_snapshot(state: &Arc<AppState>, refresh: MarketRefresh) -> MarketSnapshot {
    let orders = match &refresh.orders {
        Some(orders) => market::order_rows(state, orders).await,
        None => None,
    };
    state
        .listings
        .remember(&state.core, orders.as_ref(), refresh.auctions.as_deref());

    MarketSnapshot {
        orders,
        auctions: refresh.auctions,
        at: Utc::now(),
    }
}
