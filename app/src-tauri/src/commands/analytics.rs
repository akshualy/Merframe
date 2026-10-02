use wf_core::MarketWindow;

use super::market::unlisted_items;
use super::{Shared, ready, since};
use crate::analytics::{self, MarketMover, TradeAnalytics};
use crate::error::CommandResult;
use crate::market;
use crate::state::lock;

#[tauri::command]
pub async fn market_movers(
    state: Shared<'_>,
    window: MarketWindow,
) -> CommandResult<Vec<MarketMover>> {
    let state = ready(&state).await?;
    let table = market::item_table(&state)
        .await
        .ok_or_else(unlisted_items)?;
    let turnover = state.prices.turnover(window);
    let core = lock(&state.core);
    Ok(analytics::market_movers(&table, core.catalog(), &turnover))
}

#[tauri::command]
pub async fn trade_analytics(
    state: Shared<'_>,
    since_ms: Option<i64>,
) -> CommandResult<TradeAnalytics> {
    let state = ready(&state).await?;
    let table = market::item_table(&state)
        .await
        .ok_or_else(unlisted_items)?;
    let core = lock(&state.core);
    let trades = core.store().trades(since(since_ms))?;
    Ok(analytics::trade_analytics(&trades, &table, core.catalog()))
}
