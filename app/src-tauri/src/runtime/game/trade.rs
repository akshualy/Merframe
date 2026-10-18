use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Runtime};
use tokio::sync::watch;
use tracing::{debug, warn};
use wf_core::{ScannedTrade, ScannedTradeItem};
use wf_mem::MemoryReader;
use wf_scan::{LuaState, TradeScreen};

use super::{core_events, still_open, ticks};
use crate::runtime::attach::{Attached, attach_or_warn, read_screen};
use crate::state::AppState;

const TRADE_TICK: Duration = Duration::from_millis(50);
const TRADE_WATCH_LIFETIME: Duration = Duration::from_secs(600);

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

pub(crate) async fn trade_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let mut open = state.trade_screen_open.subscribe();
    while open.changed().await.is_ok() {
        if *open.borrow_and_update() {
            watch_trade(&app, &state, &mut open).await;
        }
    }
}

async fn completed_trade(
    reader: &Arc<dyn MemoryReader>,
    lua: LuaState,
    open: &mut watch::Receiver<bool>,
) -> Option<ScannedTrade> {
    let mut tick = ticks(TRADE_TICK);
    let mut script = None;
    while still_open(open, &mut tick).await {
        let trade = read_screen(
            "read trade screen",
            reader,
            lua,
            &mut script,
            TradeScreen::read,
        )
        .await
        .flatten();
        if let Some(trade) = trade.filter(|trade| trade.completed) {
            debug!(?trade, "Trade screen read");
            return Some(scanned_trade(trade));
        }
    }
    None
}

async fn watch_trade<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    open: &mut watch::Receiver<bool>,
) {
    let Some(Attached { reader, lua }) =
        attach_or_warn(state, "Trade screen not watched, the trade is not recorded").await
    else {
        return;
    };

    let completed = completed_trade(&reader, lua, open);
    let screen = match tokio::time::timeout(TRADE_WATCH_LIFETIME, completed).await {
        Ok(Some(screen)) => screen,
        Ok(None) => {
            debug!("Trade screen closed without a completed trade");
            return;
        }
        Err(_) => {
            debug!(
                seconds = TRADE_WATCH_LIFETIME.as_secs(),
                "Trade screen watched to the limit without a completed trade"
            );
            return;
        }
    };

    let handled = core_events(app, state, "completed trade", move |core| {
        core.handle_trade_screen(&screen, Utc::now())
    })
    .await;
    if let Err(error) = handled {
        warn!(%error, "Core rejected the completed trade");
    }
}
