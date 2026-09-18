use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use wf_core::{ListingChoices, riven_listing_payload};
use wf_market::{
    Auction, CreateOrderRequest, Order, OrderBook, OrderType, Platform, Transaction,
    UpdateAuctionRequest, UpdateOrderRequest, UserStatus,
};

use super::{Shared, missing_inventory, ready};

use crate::error::{CommandError, CommandResult};
use crate::market::{self, OrderRow, Presence};
use crate::runtime;
use crate::settings::{self, MarketAccount};
use crate::state::{lock, read, write};

fn unlisted_items() -> CommandError {
    CommandError::from("The warframe.market item list is not loaded")
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketItem {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub thumb: String,
    pub ducats: Option<u32>,
    pub tradable: Option<bool>,
    pub bulk_tradable: Option<bool>,
    pub max_rank: Option<u32>,
    pub tags: Vec<String>,
    pub subtypes: Vec<String>,
    pub max_amber_stars: Option<u32>,
    pub max_cyan_stars: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewOrder {
    pub item_id: String,
    pub order_type: OrderType,
    pub platinum: u32,
    pub quantity: u32,
    pub visible: Option<bool>,
    pub per_trade: Option<u32>,
    pub rank: Option<u32>,
    pub subtype: Option<String>,
    pub amber_stars: Option<u32>,
    pub cyan_stars: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderPatch {
    pub platinum: Option<u32>,
    pub quantity: Option<u32>,
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuctionPatch {
    pub starting_price: Option<u32>,
    pub buyout_price: Option<u32>,
    pub note: Option<String>,
    pub visible: Option<bool>,
}

#[tauri::command]
pub async fn market_login<R: Runtime>(
    app: AppHandle<R>,
    state: Shared<'_>,
    email: String,
    password: String,
) -> CommandResult<MarketAccount> {
    let state = ready(&state)?;
    let token = state.market().login(&email, &password).await?;
    *write(&state.market) = Arc::new(
        wf_market::Client::new(state.http.clone(), Platform::Pc).with_token(token.clone()),
    );

    let session = state.market().me().await?;
    if !session.user.verification {
        *write(&state.market) = Arc::new(wf_market::Client::new(state.http.clone(), Platform::Pc));
        return Err(CommandError::from(
            "warframe.market has not verified this account yet, confirm the email it sent you",
        ));
    }
    settings::set_token(&app, Some(&token))?;
    let account = MarketAccount {
        ingame_name: session.user.ingame_name,
        slug: session.user.slug,
        tier: session.user.tier,
        mastery_rank: session.user.mastery_rank,
    };
    settings::set_account(&app, Some(&account))?;
    write(&state.status).market_account = Some(account.clone());
    runtime::emit(&app, runtime::STATUS_UPDATED, state.status_snapshot());
    Ok(account)
}

#[tauri::command]
pub async fn market_logout<R: Runtime>(app: AppHandle<R>, state: Shared<'_>) -> CommandResult<()> {
    let state = ready(&state)?;
    settings::set_token(&app, None)?;
    settings::set_account(&app, None)?;
    *write(&state.market) = Arc::new(wf_market::Client::new(state.http.clone(), Platform::Pc));
    let mut status = write(&state.status);
    status.market_account = None;
    status.market_unread = 0;
    drop(status);
    market::remember_listings(&state, Some(Vec::new()), Some(Vec::new()));
    runtime::emit(&app, runtime::STATUS_UPDATED, state.status_snapshot());
    Ok(())
}

#[tauri::command]
pub async fn market_my_orders(state: Shared<'_>) -> CommandResult<Vec<OrderRow>> {
    let state = ready(&state)?;
    let orders = state.market().orders_my().await?;
    let rows = market::order_rows(&state, &orders)
        .await
        .ok_or_else(unlisted_items)?;
    market::remember_listings(&state, Some(rows.clone()), None);
    Ok(rows)
}

#[tauri::command]
pub async fn market_post_order(state: Shared<'_>, order: NewOrder) -> CommandResult<Order> {
    let state = ready(&state)?;
    let body = CreateOrderRequest {
        item_id: order.item_id,
        order_type: order.order_type,
        platinum: order.platinum,
        quantity: order.quantity,
        visible: order.visible,
        per_trade: order.per_trade,
        rank: order.rank,
        subtype: order.subtype,
        amber_stars: order.amber_stars,
        cyan_stars: order.cyan_stars,
    };
    Ok(state.market().create_order(&body).await?)
}

#[tauri::command]
pub async fn market_update_order(
    state: Shared<'_>,
    id: String,
    patch: OrderPatch,
) -> CommandResult<Order> {
    let state = ready(&state)?;
    let body = UpdateOrderRequest {
        platinum: patch.platinum,
        quantity: patch.quantity,
        visible: patch.visible,
    };
    Ok(state.market().update_order(&id, &body).await?)
}

#[tauri::command]
pub async fn market_close_order(
    state: Shared<'_>,
    id: String,
    quantity: u32,
) -> CommandResult<Transaction> {
    let state = ready(&state)?;
    Ok(state.market().close_order(&id, quantity).await?)
}

#[tauri::command]
pub async fn market_delete_order(state: Shared<'_>, id: String) -> CommandResult<Order> {
    let state = ready(&state)?;
    Ok(state.market().delete_order(&id).await?)
}

#[tauri::command]
pub async fn market_presence(state: Shared<'_>) -> CommandResult<Presence> {
    let state = ready(&state)?;
    Ok(*read(&state.market_presence))
}

#[tauri::command]
pub async fn market_set_presence(
    state: Shared<'_>,
    status: Option<UserStatus>,
    auto: bool,
) -> CommandResult<Presence> {
    let state = ready(&state)?;
    let presence = Presence { status, auto };
    *write(&state.market_presence) = presence;
    state.market_presence_wake.notify_one();
    Ok(presence)
}

#[tauri::command]
pub async fn market_activity(state: Shared<'_>) -> CommandResult<()> {
    let state = ready(&state)?;
    *lock(&state.market_activity) = Some(Instant::now());
    Ok(())
}

#[tauri::command]
pub async fn market_remove_all(state: Shared<'_>) -> CommandResult<u32> {
    let state = ready(&state)?;
    let client = state.market();
    let orders = client.orders_my().await?;
    let mut closed = 0;
    for order in orders {
        match client.delete_order(&order.id).await {
            Ok(_) => closed += 1,
            Err(error) => {
                tracing::warn!(
                    order = order.id,
                    %error,
                    "Order delete failed",
                );
            }
        }
    }
    Ok(closed)
}

#[tauri::command]
pub async fn market_fix_orders(state: Shared<'_>, id: Option<String>) -> CommandResult<u32> {
    let state = ready(&state)?;
    let client = state.market();
    let orders = client.orders_my().await?;
    let rows = market::order_rows(&state, &orders)
        .await
        .ok_or_else(unlisted_items)?;
    let wanted = rows
        .iter()
        .filter(|row| row.show_warning)
        .filter(|row| id.as_ref().is_none_or(|wanted| wanted == &row.id));
    let mut fixed = 0;
    for row in wanted {
        let repaired = match u32::try_from(row.owned) {
            Ok(0) => client.delete_order(&row.id).await.map(|_| ()),
            Ok(owned) => {
                let patch = UpdateOrderRequest {
                    quantity: Some(owned),
                    ..UpdateOrderRequest::default()
                };
                client.update_order(&row.id, &patch).await.map(|_| ())
            }
            Err(_) => continue,
        };
        match repaired {
            Ok(()) => fixed += 1,
            Err(error) => {
                tracing::warn!(
                    order = row.id,
                    owned = row.owned,
                    %error,
                    "Order quantity fix failed",
                );
            }
        }
    }
    Ok(fixed)
}

#[tauri::command]
pub async fn market_set_visibility(state: Shared<'_>, visible: bool) -> CommandResult<u32> {
    let state = ready(&state)?;
    Ok(state
        .market()
        .set_all_orders_visibility(visible)
        .await?
        .updated)
}

#[tauri::command]
pub async fn market_items(state: Shared<'_>) -> CommandResult<Vec<MarketItem>> {
    let state = ready(&state)?;
    let table = market::item_table(&state)
        .await
        .ok_or_else(unlisted_items)?;
    let mut items: Vec<MarketItem> = table
        .items()
        .iter()
        .map(|item| MarketItem {
            name: market::english_name(item),
            thumb: market::english_thumb(item),
            id: item.id.clone(),
            slug: item.slug.clone(),
            ducats: item.ducats,
            tradable: item.tradable,
            bulk_tradable: item.bulk_tradable,
            max_rank: item.max_rank,
            tags: item.tags.clone(),
            subtypes: item.subtypes.clone().unwrap_or_default(),
            max_amber_stars: item.max_amber_stars,
            max_cyan_stars: item.max_cyan_stars,
        })
        .collect();
    items.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

#[tauri::command]
pub async fn market_item_orders(state: Shared<'_>, slug: String) -> CommandResult<OrderBook> {
    let state = ready(&state)?;
    Ok(state.market().order_book(&slug).await?)
}

#[tauri::command]
pub async fn market_post_riven(
    state: Shared<'_>,
    item_id: String,
    choices: ListingChoices,
) -> CommandResult<String> {
    let state = ready(&state)?;
    let payload = {
        let core = lock(&state.core);
        let tab = core.rivens_tab().ok_or_else(missing_inventory)?;
        let row = tab
            .unveiled
            .into_iter()
            .find(|row| row.item_id == item_id)
            .ok_or_else(|| CommandError::from("This riven is not in the current inventory"))?;
        riven_listing_payload(&row, &choices).ok_or_else(|| {
            CommandError::from("This riven has no warframe.market attribute mapping")
        })?
    };
    Ok(state.market().create_auction(&payload).await?.id)
}

#[tauri::command]
pub async fn market_my_auctions(state: Shared<'_>) -> CommandResult<Vec<Auction>> {
    let state = ready(&state)?;
    let slug = read(&state.status)
        .market_account
        .as_ref()
        .map(|account| account.slug.clone())
        .ok_or_else(|| CommandError::from("Sign in to warframe.market first"))?;
    let auctions = state.market().auctions_my(&slug).await?;
    market::remember_listings(&state, None, Some(auctions.clone()));
    Ok(auctions)
}

#[tauri::command]
pub async fn market_update_auction(
    state: Shared<'_>,
    id: String,
    patch: AuctionPatch,
) -> CommandResult<Auction> {
    let state = ready(&state)?;
    let body = UpdateAuctionRequest {
        buyout_price: patch.buyout_price,
        starting_price: patch.starting_price,
        note: patch.note,
        visible: patch.visible,
    };
    Ok(state.market().update_auction(&id, &body).await?)
}

#[tauri::command]
pub async fn market_close_auction(state: Shared<'_>, id: String) -> CommandResult<Auction> {
    let state = ready(&state)?;
    Ok(state.market().close_auction(&id).await?)
}

#[tauri::command]
pub async fn market_set_auctions_visibility(state: Shared<'_>, visible: bool) -> CommandResult<()> {
    let state = ready(&state)?;
    Ok(state.market().set_auctions_visibility(visible).await?)
}
