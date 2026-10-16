use wf_core::MarketWindow;

use super::market::listed_items;
use super::{Shared, ready, since};
use crate::analytics::{self, MarketMover, TradeAnalytics};
use crate::error::CommandResult;
use crate::state::lock;

#[tauri::command]
pub async fn market_movers(
    state: Shared<'_>,
    window: MarketWindow,
) -> CommandResult<Vec<MarketMover>> {
    let state = ready(&state).await?;
    let items = listed_items(&state).await?;
    let turnover = state.prices.turnover(window);
    let core = lock(&state.core);
    Ok(analytics::market_movers(&items, core.items(), &turnover))
}

#[tauri::command]
pub async fn trade_analytics(
    state: Shared<'_>,
    since_ms: Option<i64>,
) -> CommandResult<TradeAnalytics> {
    let state = ready(&state).await?;
    let items = listed_items(&state).await?;
    let core = lock(&state.core);
    let trades = core.store().trades(since(since_ms))?;
    Ok(analytics::trade_analytics(
        &trades,
        &items,
        core.items(),
        core.catalog(),
    ))
}
