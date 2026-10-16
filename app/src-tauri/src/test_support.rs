use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use wf_core::{AlertSettings, Catalog, Core, PriceCache, Store};
use wf_market::{Item, ItemLocalization, Order, OrderType};

pub(crate) fn moment() -> DateTime<Utc> {
    DateTime::from_timestamp(1_757_410_800, 0).unwrap()
}

pub(crate) fn market_item(slug: &str, name: &str, tags: &[&str]) -> Item {
    Item {
        id: format!("id-{slug}"),
        slug: slug.to_owned(),
        game_ref: String::new(),
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        max_rank: None,
        subtypes: None,
        max_amber_stars: None,
        max_cyan_stars: None,
        tradable: Some(true),
        bulk_tradable: None,
        vaulted: None,
        ducats: None,
        rarity: None,
        i18n: HashMap::from([(
            "en".to_owned(),
            ItemLocalization {
                name: name.to_owned(),
                description: None,
                icon: String::new(),
                thumb: format!("thumbs/{slug}.png"),
            },
        )]),
    }
}

pub(crate) fn order(slug: &str, quantity: u32, rank: Option<u32>) -> Order {
    Order {
        id: format!("order-{slug}"),
        order_type: OrderType::Sell,
        platinum: 20,
        quantity,
        per_trade: None,
        subtype: None,
        rank,
        amber_stars: None,
        cyan_stars: None,
        visible: true,
        updated_at: moment(),
        item_id: format!("id-{slug}"),
        user: None,
    }
}

pub(crate) fn empty_catalog() -> Catalog {
    Catalog::from_json("[]", "[]", "[]").unwrap()
}

pub(crate) fn core_with(catalog: Catalog) -> Core {
    Core::new(
        Store::in_memory().unwrap(),
        catalog,
        Arc::new(PriceCache::default()),
        AlertSettings::default(),
    )
    .unwrap()
}
