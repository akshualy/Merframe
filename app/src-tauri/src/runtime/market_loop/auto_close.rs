use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tracing::{debug, info, warn};
use wf_core::{Trade, TradeItem, relic_refinement, traded_set};
use wf_market::{Auction, Item, Order, OrderType};

use super::{market_refresh, signed_in_slug};
use crate::market::{self, MarketCategory};
use crate::runtime::{AppEvent, emit};
use crate::state::{AppState, lock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum AutoCloseKind {
    Auction,
    Sell,
    Buy,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct MarketAutoClose {
    pub item: String,
    pub quantity: u32,
    pub kind: AutoCloseKind,
}

pub(crate) async fn auto_close<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    trade: &Trade,
) {
    let Some(slug) = signed_in_slug(state) else {
        return;
    };
    let Some((side, items, plat)) = trade_side(trade) else {
        return;
    };

    let set = traded_set(lock(&state.core).catalog(), items);
    let traded = traded_items(set.as_ref().map_or(items, std::slice::from_ref));
    if traded.is_empty() {
        return;
    }

    let refresh = market_refresh(state, &slug).await;
    let mut orders = refresh.orders.unwrap_or_default();
    let auctions = refresh.auctions.unwrap_or_default();
    let market_items = if orders.is_empty() {
        None
    } else {
        market::market_items(state).await
    };

    let client = state.market();
    for item in traded {
        if side == OrderType::Sell
            && let Some(auction) = matching_auction(&item.slug, &auctions)
        {
            match client.close_auction(&auction.id).await {
                Ok(()) => {
                    info!(
                        auction = auction.id,
                        riven = item.name,
                        "Riven auction closed after trade"
                    );
                    emit(
                        app,
                        AppEvent::MarketAutoClosed(MarketAutoClose {
                            item: item.name.clone(),
                            quantity: 1,
                            kind: AutoCloseKind::Auction,
                        }),
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

        let Some(market_items) = &market_items else {
            continue;
        };
        let Some(market_item) = market_items.listed(lock(&state.core).items(), &item.name) else {
            debug!(item = item.name, "Traded item not on warframe.market");
            continue;
        };
        let Some((order, quantity)) = matching_order(market_item, side, plat, &item, &orders)
            .map(|(order, quantity)| (order.id.clone(), quantity))
        else {
            continue;
        };

        let kind = match side {
            OrderType::Sell => AutoCloseKind::Sell,
            OrderType::Buy => AutoCloseKind::Buy,
        };
        match client.close_order(&order, quantity).await {
            Ok(()) => {
                if let Some(closed) = orders.iter_mut().find(|listed| listed.id == order) {
                    closed.quantity -= quantity;
                }
                info!(
                    order,
                    item = item.name,
                    quantity,
                    side = ?side,
                    "Order closed after trade"
                );
                emit(
                    app,
                    AppEvent::MarketAutoClosed(MarketAutoClose {
                        item: item.name.clone(),
                        quantity,
                        kind,
                    }),
                );
            }
            Err(error) => warn!(
                order,
                quantity,
                error = %error.brief(),
                "Order close after trade failed"
            ),
        }
    }
}

pub(crate) fn trade_side(trade: &Trade) -> Option<(OrderType, &[TradeItem], i64)> {
    if trade.received.is_empty() && trade.plat > 0 {
        return Some((OrderType::Sell, &trade.offered, trade.plat));
    }

    if trade.offered.is_empty() && trade.plat < 0 {
        return Some((OrderType::Buy, &trade.received, -trade.plat));
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TradedItem {
    name: String,
    slug: String,
    quantity: u32,
    rank: Option<u32>,
}

fn traded_items(items: &[TradeItem]) -> Vec<TradedItem> {
    items
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
                rank: item.rank,
            })
        })
        .collect()
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

fn matching_auction<'a>(slug: &str, auctions: &'a [Auction]) -> Option<&'a Auction> {
    auctions
        .iter()
        .find(|auction| !auction.closed && auction_slug(auction) == slug)
}

fn auction_slug(auction: &Auction) -> String {
    format!(
        "{}_{}",
        auction.item.weapon_url_name,
        trade_slug(&auction.item.name)
    )
}

fn matching_order<'a>(
    market_item: &Item,
    side: OrderType,
    plat: i64,
    item: &TradedItem,
    orders: &'a [Order],
) -> Option<(&'a Order, u32)> {
    orders
        .iter()
        .filter(|order| {
            order.order_type == side && order.item_id == market_item.id && order.quantity > 0
        })
        .min_by_key(|order| {
            let rank_distance = match (item.rank, order.rank) {
                (Some(traded), Some(listed)) => traded.abs_diff(listed),
                _ => 0,
            };
            let other_refinement = relic_refinement(&item.name)
                .is_some_and(|(_, refinement)| order.subtype.as_deref() != Some(&*refinement));

            (
                other_refinement,
                i64::from(order.platinum).abs_diff(plat),
                rank_distance,
                order.quantity,
            )
        })
        .map(|order| (order, closing_quantity(market_item, item, order)))
}

fn closing_quantity(market_item: &Item, item: &TradedItem, order: &Order) -> u32 {
    let copies = match (MarketCategory::of(market_item), item.rank) {
        (MarketCategory::Arcanes, Some(rank)) if rank > order.rank.unwrap_or(0) => {
            arcane_copies(rank)
        }
        _ => 1,
    };
    (item.quantity * copies).min(order.quantity)
}

fn arcane_copies(rank: u32) -> u32 {
    (rank + 1) * (rank + 2) / 2
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use wf_core::{Trade, TradeItem};
    use wf_market::{
        Auction, AuctionItem, ItemLocalization, Order, OrderType, Polarity, RivenAttributeInstance,
    };

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
            minimal_reputation: 0,
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
                    rank: None,
                },
                TradeItem {
                    name: "Forma Blueprint".to_owned(),
                    count: 0,
                    rank: None,
                },
            ],
            received: Vec::new(),
            plat: 45,
        };
        let (side, items, plat) = trade_side(&trade).expect("sale");
        assert_eq!(side, OrderType::Sell);
        assert_eq!(plat, 45);
        let traded = traded_items(items);
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

    fn market_item(id: &str, tags: &[&str]) -> Item {
        named_item(id, "", tags)
    }

    fn named_item(id: &str, name: &str, tags: &[&str]) -> Item {
        let en = ItemLocalization {
            name: name.to_owned(),
            description: None,
            icon: String::new(),
            thumb: String::new(),
        };
        Item {
            id: id.to_owned(),
            slug: id.to_owned(),
            game_ref: String::new(),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            max_rank: None,
            subtypes: None,
            max_amber_stars: None,
            max_cyan_stars: None,
            tradable: None,
            bulk_tradable: None,
            vaulted: None,
            ducats: None,
            rarity: None,
            i18n: HashMap::from([(String::from("en"), en)]),
        }
    }

    fn traded(quantity: u32, rank: Option<u32>) -> TradedItem {
        TradedItem {
            name: String::new(),
            slug: String::new(),
            quantity,
            rank,
        }
    }

    #[test]
    fn mixed_trade_closes_nothing() {
        let item = |name: &str| TradeItem {
            name: name.to_owned(),
            count: 1,
            rank: None,
        };
        let mixed = Trade {
            offered: vec![item("Octavia Prime Systems")],
            received: vec![item("Primed Continuity")],
            plat: 10,
        };
        assert!(trade_side(&mixed).is_none());
        let purchase = Trade {
            offered: Vec::new(),
            received: vec![item("Primed Continuity")],
            plat: -120,
        };
        let (side, _, plat) = trade_side(&purchase).expect("purchase");
        assert_eq!(side, OrderType::Buy);
        assert_eq!(plat, 120);
    }

    #[test]
    fn relic_refinement_wins_over_platinum() {
        let mut radiant = sell_order("radiant", "item-relic", 1);
        radiant.subtype = Some("radiant".to_owned());
        radiant.platinum = 10;
        let mut intact = sell_order("intact", "item-relic", 1);
        intact.subtype = Some("intact".to_owned());
        intact.platinum = 40;
        let orders = vec![radiant, intact];
        let mut item = traded(1, None);
        item.name = "Axi D6 Relic".to_owned();
        let (order, _) = matching_order(
            &market_item("item-relic", &[]),
            OrderType::Sell,
            12,
            &item,
            &orders,
        )
        .expect("sell order");
        assert_eq!(order.id, "intact");
        item.name = "Axi D6 Relic [RADIANT]".to_owned();
        let (order, _) = matching_order(
            &market_item("item-relic", &[]),
            OrderType::Sell,
            40,
            &item,
            &orders,
        )
        .expect("sell order");
        assert_eq!(order.id, "radiant");
    }

    #[test]
    fn closest_platinum_wins() {
        let mut cheap = sell_order("cheap", "item-octavia", 1);
        cheap.platinum = 10;
        let mut dear = sell_order("dear", "item-octavia", 1);
        dear.platinum = 40;
        let orders = vec![cheap, dear];
        let (order, _) = matching_order(
            &market_item("item-octavia", &[]),
            OrderType::Sell,
            35,
            &traded(1, None),
            &orders,
        )
        .expect("sell order");
        assert_eq!(order.id, "dear");
    }

    #[test]
    fn sell_order_for_traded_quantity() {
        let orders = vec![
            buy_order("buy", "item-octavia"),
            sell_order("sell", "item-octavia", 3),
        ];
        let (order, quantity) = matching_order(
            &market_item("item-octavia", &[]),
            OrderType::Sell,
            22,
            &traded(2, None),
            &orders,
        )
        .expect("sell order");
        assert_eq!(order.id, "sell");
        assert_eq!(quantity, 2);
    }

    #[test]
    fn closest_rank_wins() {
        let mut maxed = sell_order("maxed", "item-mod", 1);
        maxed.rank = Some(10);
        let mut unranked = sell_order("unranked", "item-mod", 1);
        unranked.rank = Some(0);
        let orders = vec![maxed, unranked];
        let (order, _) = matching_order(
            &market_item("item-mod", &[]),
            OrderType::Sell,
            22,
            &traded(1, Some(0)),
            &orders,
        )
        .expect("sell order");
        assert_eq!(order.id, "unranked");
        let (order, _) = matching_order(
            &market_item("item-mod", &[]),
            OrderType::Sell,
            22,
            &traded(1, None),
            &orders,
        )
        .expect("sell order");
        assert_eq!(order.id, "maxed");
    }

    #[test]
    fn ranked_arcane_closes_its_copies() {
        let mut unranked = sell_order("unranked", "item-arcane", 20);
        unranked.rank = Some(0);
        let orders = vec![unranked];
        let arcane = market_item("item-arcane", &["arcane_enhancement"]);
        let (_, quantity) =
            matching_order(&arcane, OrderType::Sell, 42, &traded(1, Some(3)), &orders)
                .expect("sell order");
        assert_eq!(quantity, 10);
        let (_, quantity) =
            matching_order(&arcane, OrderType::Sell, 42, &traded(2, Some(0)), &orders)
                .expect("sell order");
        assert_eq!(quantity, 2);
        let mut maxed = sell_order("maxed", "item-arcane", 1);
        maxed.rank = Some(5);
        let orders = vec![maxed];
        let (order, quantity) =
            matching_order(&arcane, OrderType::Sell, 42, &traded(1, Some(5)), &orders)
                .expect("sell order");
        assert_eq!(order.id, "maxed");
        assert_eq!(quantity, 1);
        assert_eq!(arcane_copies(1), 3);
        assert_eq!(arcane_copies(5), 21);
    }

    #[test]
    fn quantity_capped_at_order() {
        let orders = vec![sell_order("sell", "item-octavia", 1)];
        let (_, quantity) = matching_order(
            &market_item("item-octavia", &[]),
            OrderType::Sell,
            22,
            &traded(6, None),
            &orders,
        )
        .expect("sell order");
        assert_eq!(quantity, 1);
    }

    #[test]
    fn buy_order_never_matches() {
        let orders = vec![buy_order("buy", "item-octavia")];
        assert!(
            matching_order(
                &market_item("item-octavia", &[]),
                OrderType::Sell,
                22,
                &traded(1, None),
                &orders
            )
            .is_none()
        );
        assert!(
            matching_order(
                &market_item("item-mirage", &[]),
                OrderType::Sell,
                22,
                &traded(1, None),
                &orders
            )
            .is_none()
        );
    }
}
