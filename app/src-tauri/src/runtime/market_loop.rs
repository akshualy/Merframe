use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tracing::{debug, info, warn};
use wf_core::Trade;
use wf_market::{
    Auction, IncomingEvent, MarketError, MarketSocket, Order, OrderType, Platform, Session,
    StatusSetPayload, UserStatus,
};

use super::{MARKET_AUTO_CLOSED, STATUS_UPDATED, emit};
use crate::market::{self, OrderRow};
use crate::settings::{self, MarketAccount};
use crate::state::{AppState, lock, read, write};

const MARKET_SIGNED_OUT_POLL: Duration = Duration::from_secs(30);

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

#[derive(Debug, Clone, Serialize)]
pub struct MarketSnapshot {
    pub orders: Option<Vec<OrderRow>>,
    pub auctions: Option<Vec<Auction>>,
    pub at: DateTime<Utc>,
}

fn poll_backoff(interval: Duration, failures: u32) -> Duration {
    interval * (1 << failures.min(3))
}

fn poll_interval(idle: Duration, since_activity: Option<Duration>) -> Duration {
    if since_activity.is_some_and(|elapsed| elapsed < Duration::from_mins(5)) {
        Duration::from_secs(60)
    } else {
        idle
    }
}

fn signed_in_slug(state: &Arc<AppState>) -> Option<String> {
    read(&state.status)
        .market_account
        .as_ref()
        .map(|account| account.slug.clone())
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
    market::remember_listings(state, orders.clone(), refresh.auctions.clone());
    MarketSnapshot {
        orders,
        auctions: refresh.auctions,
        at: Utc::now(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketAutoClose {
    pub item: String,
    pub quantity: u32,
    pub auction: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TradedItem {
    name: String,
    slug: String,
    quantity: u32,
}

fn trade_slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut separated = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if separated && !slug.is_empty() {
                slug.push('_');
            }
            separated = false;
            slug.extend(ch.to_lowercase());
        } else {
            separated = true;
        }
    }
    slug
}

fn traded_items(trade: &Trade) -> Vec<TradedItem> {
    trade
        .offered
        .iter()
        .filter_map(|item| {
            let quantity = u32::try_from(item.count).ok()?;
            if quantity == 0 {
                return None;
            }
            Some(TradedItem {
                name: item.name.clone(),
                slug: trade_slug(&item.name),
                quantity,
            })
        })
        .collect()
}

fn auction_slug(auction: &Auction) -> String {
    format!(
        "{}_{}",
        auction.item.weapon_url_name,
        trade_slug(&auction.item.name)
    )
}

fn matching_auction<'a>(slug: &str, auctions: &'a [Auction]) -> Option<&'a Auction> {
    auctions
        .iter()
        .find(|auction| !auction.closed && auction_slug(auction) == slug)
}

fn matching_order<'a>(
    item_id: &str,
    quantity: u32,
    orders: &'a [Order],
) -> Option<(&'a Order, u32)> {
    orders
        .iter()
        .find(|order| {
            order.order_type == OrderType::Sell && order.item_id == item_id && order.quantity > 0
        })
        .map(|order| (order, quantity.min(order.quantity)))
}

pub(super) async fn auto_close<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    trade: &Trade,
) {
    let Some(slug) = signed_in_slug(state) else {
        return;
    };
    let traded = traded_items(trade);
    if traded.is_empty() {
        return;
    }
    let refresh = market_refresh(state, &slug).await;
    let orders = refresh.orders.unwrap_or_default();
    let auctions = refresh.auctions.unwrap_or_default();
    let client = state.market();
    for item in traded {
        if let Some(auction) = matching_auction(&item.slug, &auctions) {
            match client.close_auction(&auction.id).await {
                Ok(_) => {
                    info!(
                        auction = auction.id,
                        riven = item.name,
                        "Riven auction closed after trade"
                    );
                    emit(
                        app,
                        MARKET_AUTO_CLOSED,
                        MarketAutoClose {
                            item: item.name.clone(),
                            quantity: 1,
                            auction: true,
                        },
                    );
                }
                Err(error) => warn!(
                    auction = auction.id,
                    error = %error.brief(),
                    "Auction close after trade failed"
                ),
            }
            continue;
        }
        if orders.is_empty() {
            continue;
        }
        let market_item = match client.item(&item.slug).await {
            Ok(market_item) => market_item,
            Err(error) => {
                debug!(
                    slug = item.slug,
                    error = %error.brief(),
                    "Traded item not on warframe.market"
                );
                continue;
            }
        };
        let Some((order, quantity)) = matching_order(&market_item.id, item.quantity, &orders)
        else {
            continue;
        };
        match client.close_order(&order.id, quantity).await {
            Ok(()) => {
                info!(
                    order = order.id,
                    item = item.name,
                    quantity,
                    "Sell order closed after trade"
                );
                emit(
                    app,
                    MARKET_AUTO_CLOSED,
                    MarketAutoClose {
                        item: item.name.clone(),
                        quantity,
                        auction: false,
                    },
                );
            }
            Err(error) => warn!(
                order = order.id,
                quantity,
                error = %error.brief(),
                "Sell order close after trade failed"
            ),
        }
    }
}

async fn refresh_unread<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let chats = match state.market().chats().await {
        Ok(chats) => chats,
        Err(error) => return refresh_failed("messages", &error),
    };
    let unread = chats.iter().map(|chat| chat.unread_count).sum();
    if read(&state.status).market_unread != unread {
        write(&state.status).market_unread = unread;
        emit(app, STATUS_UPDATED, state.status_snapshot());
    }
}

fn sign_out<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    if let Err(error) = settings::set_token(app, None) {
        warn!(%error, "Clearing the stored market token failed");
    }
    if let Err(error) = settings::set_account(app, None) {
        warn!(%error, "Clearing the stored market account failed");
    }
    *write(&state.market) = Arc::new(wf_market::Client::new(state.http.clone(), Platform::Pc));
    let mut status = write(&state.status);
    status.market_account = None;
    status.market_unread = 0;
    drop(status);
    emit(app, STATUS_UPDATED, state.status_snapshot());
    emit(app, "market-signed-out", ());
}

fn account_of(session: &Session) -> MarketAccount {
    MarketAccount {
        ingame_name: session.user.ingame_name.clone(),
        slug: session.user.slug.clone(),
        tier: session.user.tier.clone(),
        mastery_rank: session.user.mastery_rank,
    }
}

async fn checked_session<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    was_signed_in: bool,
) -> Option<String> {
    let session = match state.market().me().await {
        Ok(session) => session,
        Err(MarketError::Unauthorized) => {
            info!("Warframe.market rejected the stored token");
            sign_out(app, state);
            return None;
        }
        Err(error) => {
            warn!(error = %error.brief(), "Warframe.market session check failed");
            return None;
        }
    };
    if let Some(token) = &session.rotated_token {
        debug!("Warframe.market issued a new token");
        if let Err(error) = settings::set_token(app, Some(token)) {
            warn!(
                %error,
                "Rotated token save failed",
            );
        }
        *write(&state.market) = Arc::new(
            wf_market::Client::new(state.http.clone(), Platform::Pc).with_token(token.clone()),
        );
    }
    let account = account_of(&session);
    if !was_signed_in {
        info!(account = account.ingame_name, "Warframe.market signed in");
        if let Err(error) = settings::set_account(app, Some(&account)) {
            warn!(%error, "Market account save failed");
        }
        write(&state.status).market_account = Some(account.clone());
        emit(app, STATUS_UPDATED, state.status_snapshot());
    }
    Some(account.slug)
}

pub(super) async fn market_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let mut failures: u32 = 0;
    let mut verified_at: Option<Instant> = None;
    loop {
        if !state.market().has_token() {
            failures = 0;
            verified_at = None;
            tokio::time::sleep(MARKET_SIGNED_OUT_POLL).await;
            continue;
        }
        let since_activity = lock(&state.market_activity).map(|touched| touched.elapsed());
        let interval = poll_interval(read(&state.settings).market_poll_interval(), since_activity);
        let checked_recently = verified_at.is_some_and(|at| at.elapsed() < Duration::from_hours(5));
        let slug = match signed_in_slug(&state) {
            Some(slug) if checked_recently => Some(slug),
            known => {
                let checked = checked_session(&app, &state, known.is_some()).await;
                verified_at = checked.as_ref().map(|_| Instant::now());
                checked
            }
        };
        let Some(slug) = slug else {
            tokio::time::sleep(interval).await;
            continue;
        };
        refresh_unread(&app, &state).await;
        let refresh = market_refresh(&state, &slug).await;
        if refresh.is_complete() {
            failures = 0;
        } else {
            failures = failures.saturating_add(1);
        }
        if !refresh.is_empty() {
            let snapshot = market_snapshot(&state, refresh).await;
            debug!(
                orders = snapshot.orders.as_ref().map(Vec::len),
                auctions = snapshot.auctions.as_ref().map(Vec::len),
                "Market listings refreshed"
            );
            emit(&app, "market-updated", snapshot);
        }
        tokio::time::sleep(poll_backoff(interval, failures)).await;
    }
}

fn socket_backoff(failures: u32) -> Duration {
    Duration::from_secs(10)
        .saturating_mul(1 << failures.min(6))
        .min(Duration::from_mins(10))
}

fn wanted_presence(state: &Arc<AppState>) -> Option<UserStatus> {
    let detected = read(&state.status).game_detected;
    read(&state.market_presence).wanted(detected)
}

enum Handshake {
    Accepted(Box<MarketSocket>),
    Rejected,
    Closed,
}

async fn handshake(token: String) -> Handshake {
    match tokio::time::timeout(Duration::from_secs(15), authenticated(token)).await {
        Ok(Ok(handshake)) => handshake,
        Ok(Err(error)) => {
            warn!(%error, "Market socket handshake failed");
            Handshake::Closed
        }
        Err(_) => {
            warn!("Market socket did not acknowledge the token in fifteen seconds");
            Handshake::Closed
        }
    }
}

async fn authenticated(token: String) -> wf_market::Result<Handshake> {
    let mut socket = MarketSocket::connect().await?;
    socket.authenticate(token).await?;
    loop {
        match socket.next_event().await? {
            Some(IncomingEvent::AuthOk) => {
                debug!("Market socket authenticated");
                return Ok(Handshake::Accepted(Box::new(socket)));
            }
            Some(IncomingEvent::AuthError) => return Ok(Handshake::Rejected),
            Some(_) => {}
            None => return Ok(Handshake::Closed),
        }
    }
}

async fn presence_session<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    socket: &mut MarketSocket,
) -> wf_market::Result<()> {
    let mut reported: Option<UserStatus> = None;
    let mut last_frame = Instant::now();
    loop {
        let wanted = wanted_presence(state);
        if let Some(status) = wanted
            && reported != wanted
        {
            socket
                .set_status(StatusSetPayload {
                    status,
                    duration: None,
                })
                .await?;
            reported = wanted;
            info!(status = ?status, "Market socket status set");
        }
        tokio::select! {
            event = socket.next_event() => {
                last_frame = Instant::now();
                match event? {
                    Some(IncomingEvent::AuthError) => {
                        info!("Market socket rejected the session, signing out");
                        sign_out(app, state);
                        return Ok(());
                    }
                    Some(IncomingEvent::StatusSet { payload, .. }) => {
                        reported = Some(payload.status);
                        emit(app, "market-presence", payload.status);
                    }
                    Some(_) => {}
                    None => return Ok(()),
                }
            },
            () = state.market_presence_wake.notified() => {}
            () = tokio::time::sleep(Duration::from_secs(15)) => {
                if last_frame.elapsed() >= Duration::from_mins(3) {
                    info!("Market socket silent for three minutes, reconnecting");
                    return Ok(());
                }
            }
        }
    }
}

pub(super) async fn market_presence_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let mut failures: u32 = 0;
    loop {
        let token = match settings::token(&app) {
            Ok(Some(token)) if signed_in_slug(&state).is_some() => token,
            Ok(_) => {
                tokio::time::sleep(MARKET_SIGNED_OUT_POLL).await;
                continue;
            }
            Err(error) => {
                warn!(%error, "Stored market token unreadable, presence task stopped");
                return;
            }
        };
        match handshake(token).await {
            Handshake::Accepted(mut socket) => {
                failures = 0;
                if let Err(error) = presence_session(&app, &state, &mut socket).await {
                    warn!(%error, "Market socket dropped");
                }
            }
            Handshake::Rejected => {
                info!("Market socket rejected the stored token");
                sign_out(&app, &state);
            }
            Handshake::Closed => failures = failures.saturating_add(1),
        }
        tokio::time::sleep(socket_backoff(failures)).await;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;
    use wf_core::{Trade, TradeItem};
    use wf_market::{Auction, AuctionItem, Order, OrderType, Polarity, RivenAttributeInstance};

    use super::*;

    fn sell_order(id: &str, item_id: &str, quantity: u32) -> Order {
        Order {
            id: id.to_owned(),
            order_type: OrderType::Sell,
            platinum: 22,
            quantity,
            per_trade: None,
            subtype: None,
            rank: None,
            amber_stars: None,
            cyan_stars: None,
            visible: true,
            updated_at: Utc::now(),
            item_id: item_id.to_owned(),
            user: None,
        }
    }

    fn buy_order(id: &str, item_id: &str) -> Order {
        Order {
            order_type: OrderType::Buy,
            ..sell_order(id, item_id, 3)
        }
    }

    fn auction(id: &str, weapon: &str, name: &str, closed: bool) -> Auction {
        Auction {
            id: id.to_owned(),
            buyout_price: Some(500),
            starting_price: 500,
            note: None,
            item: AuctionItem {
                attributes: vec![RivenAttributeInstance {
                    value: 124.4,
                    positive: true,
                    url_name: "electric_damage".to_owned(),
                }],
                polarity: Polarity::Naramon,
                mod_rank: 8,
                name: name.to_owned(),
                re_rolls: 86,
                mastery_level: 12,
                weapon_url_name: weapon.to_owned(),
            },
            private: false,
            visible: true,
            closed,
            is_direct_sell: true,
            created: String::from("2026-09-07T18:55:39.000+00:00"),
            updated: String::from("2026-09-07T18:55:39.000+00:00"),
        }
    }

    #[test]
    fn trade_slugs() {
        assert_eq!(trade_slug("Octavia Prime Systems"), "octavia_prime_systems");
        assert_eq!(trade_slug("Primed Continuity"), "primed_continuity");
        assert_eq!(trade_slug("Okina Acri-Vexicak"), "okina_acri_vexicak");
        assert_eq!(trade_slug("Axi A1 Relic"), "axi_a1_relic");
    }

    #[test]
    fn traded_items_from_offer() {
        let trade = Trade {
            offered: vec![
                TradeItem {
                    name: "Octavia Prime Systems".to_owned(),
                    count: 2,
                },
                TradeItem {
                    name: "Forma Blueprint".to_owned(),
                    count: 0,
                },
            ],
            received: vec![TradeItem {
                name: "Primed Continuity".to_owned(),
                count: 1,
            }],
            plat: 45,
        };
        let traded = traded_items(&trade);
        assert_eq!(traded.len(), 1);
        assert_eq!(traded[0].slug, "octavia_prime_systems");
        assert_eq!(traded[0].name, "Octavia Prime Systems");
        assert_eq!(traded[0].quantity, 2);
    }

    #[test]
    fn open_auction_by_slug() {
        let auctions = vec![
            auction("closed-one", "okina", "acri-vexicak", true),
            auction("open-one", "okina", "acri-vexicak", false),
            auction("other-weapon", "kuva_bramma", "crita-critacan", false),
        ];
        assert_eq!(auction_slug(&auctions[1]), "okina_acri_vexicak");
        let slug = trade_slug("Okina Acri-Vexicak");
        let matched = matching_auction(&slug, &auctions).expect("open auction");
        assert_eq!(matched.id, "open-one");
        assert!(matching_auction("okina_visi_visitio", &auctions).is_none());
    }

    #[test]
    fn sell_order_for_traded_quantity() {
        let orders = vec![
            buy_order("buy", "item-octavia"),
            sell_order("sell", "item-octavia", 3),
        ];
        let (order, quantity) = matching_order("item-octavia", 2, &orders).expect("sell order");
        assert_eq!(order.id, "sell");
        assert_eq!(quantity, 2);
    }

    #[test]
    fn quantity_capped_at_order() {
        let orders = vec![sell_order("sell", "item-octavia", 1)];
        let (_, quantity) = matching_order("item-octavia", 6, &orders).expect("sell order");
        assert_eq!(quantity, 1);
    }

    #[test]
    fn buy_order_never_matches() {
        let orders = vec![buy_order("buy", "item-octavia")];
        assert!(matching_order("item-octavia", 1, &orders).is_none());
        assert!(matching_order("item-mirage", 1, &orders).is_none());
    }

    #[test]
    fn socket_backoff_caps_at_ten_minutes() {
        let sequence: Vec<u64> = (0..9)
            .map(|failures| socket_backoff(failures).as_secs())
            .collect();
        assert_eq!(sequence, [10, 20, 40, 80, 160, 320, 600, 600, 600]);
        assert_eq!(socket_backoff(99), Duration::from_mins(10));
    }

    #[test]
    fn active_page_polls_every_minute() {
        let idle = Duration::from_secs(300);
        assert_eq!(poll_interval(idle, None), idle);
        assert_eq!(
            poll_interval(idle, Some(Duration::from_secs(10))),
            Duration::from_secs(60)
        );
        assert_eq!(
            poll_interval(idle, Some(Duration::from_secs(299))),
            Duration::from_secs(60)
        );
        assert_eq!(poll_interval(idle, Some(Duration::from_secs(301))), idle);
    }

    #[test]
    fn no_failures_no_backoff() {
        let interval = Duration::from_secs(300);
        assert_eq!(poll_backoff(interval, 0), interval);
    }

    #[test]
    fn poll_backoff_doubles_to_eight() {
        let interval = Duration::from_secs(300);
        assert_eq!(poll_backoff(interval, 1), interval * 2);
        assert_eq!(poll_backoff(interval, 2), interval * 4);
        assert_eq!(poll_backoff(interval, 3), interval * 8);
        assert_eq!(poll_backoff(interval, 9), interval * 8);
        assert_eq!(poll_backoff(interval, 9).as_secs(), 2400);
    }
}
