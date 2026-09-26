use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use wf_core::Catalog;
use wf_data::store_item_to_type;
use wf_inventory::Inventory;
use wf_worldstate::{
    ArchonHunt, BaroStatus, Circuit, DailyDeal, Fissure, MarketSale, NightwaveSeason, Sortie,
    Timer, Varzia, WorldState,
};

use super::{Shared, ready};
use crate::error::CommandResult;
use crate::state::{lock, read};

#[derive(Debug, Clone, Serialize)]
pub struct WorldStateView {
    pub fissures: Vec<Fissure>,
    pub baro: Option<BaroStatus>,
    pub baro_manifest: Vec<BaroGroup>,
    pub sortie: Option<Sortie>,
    pub archon_hunt: Option<ArchonHunt>,
    pub timers: Vec<Timer>,
    pub daily_deals: Vec<DarvoDeal>,
    pub market_sales: Vec<MarketOffer>,
    pub circuit: Option<Circuit>,
    pub prime_resurgence: Option<PrimeResurgence>,
    pub nightwave: Option<NightwaveSeason>,
    pub fetched_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BaroOffer {
    pub name: String,
    pub ducats: Option<u32>,
    pub credits: Option<u32>,
    pub owned: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BaroGroup {
    pub name: &'static str,
    pub items: Vec<BaroOffer>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DarvoDeal {
    pub name: String,
    pub discount_percent: u32,
    pub original_price: u32,
    pub sale_price: u32,
    pub amount_total: u32,
    pub amount_sold: u32,
    pub expiry: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketOffer {
    pub name: String,
    pub discount_percent: u32,
    pub platinum: u32,
    pub credits: u32,
    pub ends: DateTime<Utc>,
    pub owned: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResurgenceOffering {
    pub name: String,
    pub regal_aya: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrimeResurgence {
    pub node_id: String,
    pub node_name: Option<&'static str>,
    pub ends: DateTime<Utc>,
    pub offerings: Vec<ResurgenceOffering>,
}

fn named_circuit(catalog: &Catalog, circuit: &Circuit) -> Circuit {
    let by_squashed_name: HashMap<String, &str> = catalog
        .items()
        .map(|item| {
            let squashed = item.name.replace(' ', "").replace('&', "And");
            (squashed, item.name.as_str())
        })
        .collect();
    let named = |choices: &[String]| -> Vec<String> {
        choices
            .iter()
            .map(|choice| match by_squashed_name.get(choice) {
                Some(name) => (*name).to_owned(),
                None => choice.clone(),
            })
            .collect()
    };
    Circuit {
        rotates: circuit.rotates,
        normal: named(&circuit.normal),
        hard: named(&circuit.hard),
    }
}

fn baro_group_of(item: Option<&wf_data::Item>) -> &'static str {
    match item.map(|item| item.category.as_str()) {
        Some("Mods") => "Mods",
        Some(
            "Warframes" | "Archwing" | "Primary" | "Secondary" | "Melee" | "Arch-Gun"
            | "Arch-Melee",
        ) => "Weapons",
        Some("Skins" | "Sigils") => "Cosmetics",
        _ => "Others",
    }
}

fn baro_offer_name(catalog: &Catalog, item_type: &str) -> String {
    if let Some((relic, _)) = catalog.relic_by_unique_name(item_type) {
        return format!("{} Relic", relic.name);
    }
    if item_type.ends_with("/MummyQuestKeyBlueprint") {
        return "Sands of Inaros Blueprint".to_owned();
    }
    if item_type.contains("/AvatarImages/") {
        return "Icon".to_owned();
    }
    if item_type.contains("/Boosters/") {
        return "Booster".to_owned();
    }
    "Other".to_owned()
}

fn baro_manifest(
    catalog: &Catalog,
    inventory: Option<&Inventory>,
    baro: Option<&BaroStatus>,
) -> Vec<BaroGroup> {
    let Some(BaroStatus::Present { items, .. }) = baro else {
        return Vec::new();
    };
    let mut groups: Vec<BaroGroup> = ["Mods", "Weapons", "Cosmetics", "Others"]
        .into_iter()
        .map(|name| BaroGroup {
            name,
            items: Vec::new(),
        })
        .collect();
    for offer in items {
        let item_type = store_item_to_type(&offer.item_type);
        if item_type.ends_with("/BaroTreasureBox") {
            continue;
        }
        let known = catalog.item(&item_type);
        let group = baro_group_of(known);
        let name = known.map_or_else(
            || baro_offer_name(catalog, &item_type),
            |item| item.name.clone(),
        );
        if let Some(target) = groups.iter_mut().find(|target| target.name == group) {
            target.items.push(BaroOffer {
                name,
                ducats: offer.ducats,
                credits: offer.credits,
                owned: inventory.map(|inventory| inventory.owns(&item_type)),
            });
        }
    }
    for group in &mut groups {
        group.items.sort_by(|a, b| a.name.cmp(&b.name));
    }
    groups.retain(|group| !group.items.is_empty());
    groups
}

fn darvo_deals(catalog: &Catalog, deals: Vec<DailyDeal>) -> Vec<DarvoDeal> {
    deals
        .into_iter()
        .map(|deal| {
            let item_type = store_item_to_type(&deal.store_item);
            let name = catalog
                .item(&item_type)
                .map_or_else(|| item_type.clone(), |item| item.name.clone());
            DarvoDeal {
                name,
                discount_percent: deal.discount_percent,
                original_price: deal.original_price,
                sale_price: deal.sale_price,
                amount_total: deal.amount_total,
                amount_sold: deal.amount_sold,
                expiry: deal.expiry,
            }
        })
        .collect()
}

fn market_offers(
    catalog: &Catalog,
    inventory: Option<&Inventory>,
    sales: Vec<MarketSale>,
) -> Vec<MarketOffer> {
    let mut offers: Vec<MarketOffer> = sales
        .into_iter()
        .filter_map(|sale| {
            let owned = match sale.package_name {
                Some(_) => None,
                None => inventory.map(|inventory| inventory.owns(&sale.type_name)),
            };
            let name = sale
                .package_name
                .or_else(|| catalog.item(&sale.type_name).map(|item| item.name.clone()))?;
            Some(MarketOffer {
                name,
                discount_percent: sale.discount_percent,
                platinum: sale.platinum,
                credits: sale.credits,
                ends: sale.ends,
                owned,
            })
        })
        .collect();
    offers.sort_by(|a, b| a.ends.cmp(&b.ends).then_with(|| a.name.cmp(&b.name)));
    offers
}

fn prime_resurgence(catalog: &Catalog, varzia: Varzia) -> PrimeResurgence {
    let mut offerings: Vec<ResurgenceOffering> = varzia
        .items
        .iter()
        .filter_map(|item| {
            catalog
                .item(&store_item_to_type(&item.item_type))
                .map(|known| ResurgenceOffering {
                    name: known.name.clone(),
                    regal_aya: item.regal_aya,
                })
        })
        .collect();
    offerings.sort_by(|a, b| a.name.cmp(&b.name));
    PrimeResurgence {
        node_id: varzia.node_id,
        node_name: varzia.node_name,
        ends: varzia.expiry,
        offerings,
    }
}

impl WorldStateView {
    pub fn build(
        world: Option<&WorldState>,
        catalog: &Catalog,
        inventory: Option<&Inventory>,
        now: DateTime<Utc>,
        fetched_at: Option<DateTime<Utc>>,
    ) -> Self {
        let Some(world) = world else {
            return Self {
                fissures: Vec::new(),
                baro: None,
                baro_manifest: Vec::new(),
                sortie: None,
                archon_hunt: None,
                timers: Vec::new(),
                daily_deals: Vec::new(),
                market_sales: Vec::new(),
                circuit: None,
                prime_resurgence: None,
                nightwave: None,
                fetched_at,
            };
        };
        let baro = world.baro(now);
        Self {
            fissures: world.fissures(now),
            baro_manifest: baro_manifest(catalog, inventory, baro.as_ref()),
            baro,
            sortie: world.sortie(now),
            archon_hunt: world.archon_hunt(now),
            timers: world.timers(now),
            daily_deals: darvo_deals(catalog, world.daily_deals(now)),
            market_sales: market_offers(catalog, inventory, world.market_sales(now)),
            circuit: world
                .circuit(now)
                .map(|circuit| named_circuit(catalog, &circuit)),
            prime_resurgence: world
                .varzia(now)
                .map(|varzia| prime_resurgence(catalog, varzia)),
            nightwave: world.season(now),
            fetched_at,
        }
    }
}

#[tauri::command]
pub async fn worldstate(state: Shared<'_>) -> CommandResult<WorldStateView> {
    let state = ready(&state)?;
    let fetched_at = read(&state.status).world_state_at;
    let world = read(&state.world);
    let core = lock(&state.core);
    Ok(WorldStateView::build(
        world.as_ref(),
        core.catalog(),
        core.inventory(),
        Utc::now(),
        fetched_at,
    ))
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use wf_core::Catalog;
    use wf_worldstate::WorldState;

    use super::*;

    const FIXTURE: &str = include_str!("../../../../fixtures/worldState.json");
    const INVENTORY: &str = include_str!("../../../../fixtures/inventory.json");
    const FIXTURE_NOW_MS: i64 = 1_788_807_069_000;
    const ITEMS: &str = r#"[{
        "uniqueName": "/Lotus/Types/Items/Research/BioComponent",
        "name": "Mutagen Mass",
        "category": "Misc",
        "type": "Resource",
        "tradable": false
    }]"#;

    fn view() -> WorldStateView {
        let world = WorldState::parse(FIXTURE).unwrap();
        let catalog = Catalog::from_json(ITEMS, "[]", "[]").unwrap();
        let now = DateTime::from_timestamp_millis(FIXTURE_NOW_MS).unwrap();
        WorldStateView::build(Some(&world), &catalog, None, now, Some(now))
    }

    #[test]
    fn fixture_view_panels() {
        let view = view();
        assert!(!view.fissures.is_empty());
        assert!(!view.timers.is_empty());
        assert!(view.baro.is_some());
        assert!(view.sortie.is_some());
        assert!(view.archon_hunt.is_some());
        assert!(view.circuit.is_some());
        assert!(view.prime_resurgence.is_some());
        assert!(view.fetched_at.is_some());
    }

    #[test]
    fn storms_and_named_sortie_in_view() {
        let view = view();
        assert_eq!(
            view.fissures
                .iter()
                .filter(|fissure| fissure.is_storm)
                .count(),
            6
        );
        assert_eq!(view.sortie.unwrap().boss_name, "Kela De Thaym");
        assert_eq!(view.archon_hunt.unwrap().faction, Some("Narmer"));
    }

    #[test]
    fn darvo_deal() {
        let view = view();
        assert_eq!(view.daily_deals.len(), 1);
        let deal = &view.daily_deals[0];
        assert_eq!(deal.name, "Mutagen Mass");
        assert_eq!(deal.sale_price, 8);
        assert_eq!(deal.original_price, 10);
        assert_eq!(deal.discount_percent, 20);
        assert_eq!(deal.amount_total, 165);
        assert_eq!(deal.expiry.timestamp_millis(), 1_788_814_800_000);
    }

    #[test]
    fn market_sale_offers() {
        const CATALOG: &str = r#"[
            {"uniqueName":"/Lotus/Upgrades/Skins/Sigils/BossSigilJackal","name":"Jackal Sigil",
             "category":"Sigils","type":"Sigil","tradable":false},
            {"uniqueName":"/Lotus/Upgrades/Skins/Necro/NecroDangles","name":"Mortos Binds",
             "category":"Skins","type":"Skin","tradable":false}
        ]"#;
        const SALES: &str = r#"{"FlashSales":[
            {"TypeName":"/Lotus/Upgrades/Skins/Necro/NecroDangles","Discount":20,"PremiumOverride":40,
             "StartDate":{"$date":{"$numberLong":"1788800000000"}},
             "EndDate":{"$date":{"$numberLong":"1788900000000"}}},
            {"TypeName":"/Lotus/Upgrades/Skins/Sigils/BossSigilJackal","RegularOverride":1,
             "ProductExpiryOverride":{"$date":{"$numberLong":"1788880000000"}},
             "StartDate":{"$date":{"$numberLong":"1788800000000"}},
             "EndDate":{"$date":{"$numberLong":"1788880000000"}}},
            {"TypeName":"/Lotus/Types/StoreItems/Packages/CrpIndexTwoArmorPack","Discount":20,"PremiumOverride":80,
             "StartDate":{"$date":{"$numberLong":"1788800000000"}},
             "EndDate":{"$date":{"$numberLong":"1788890000000"}}},
            {"TypeName":"/Lotus/Upgrades/Skins/Unlisted/UnlistedSkin","Discount":20,"PremiumOverride":60,
             "StartDate":{"$date":{"$numberLong":"1788800000000"}},
             "EndDate":{"$date":{"$numberLong":"1788900000000"}}}
        ]}"#;

        let world = WorldState::parse(SALES).unwrap();
        let catalog = Catalog::from_json(CATALOG, "[]", "[]").unwrap();
        let inventory = Inventory::parse(INVENTORY).unwrap();
        let now = DateTime::from_timestamp_millis(1_788_850_000_000).unwrap();
        let view = WorldStateView::build(Some(&world), &catalog, Some(&inventory), now, None);
        let offers: Vec<(&str, u32, Option<bool>)> = view
            .market_sales
            .iter()
            .map(|offer| (offer.name.as_str(), offer.discount_percent, offer.owned))
            .collect();
        assert_eq!(
            offers,
            vec![
                ("Jackal Sigil", 0, Some(true)),
                ("Quaro Armor Collection", 20, None),
                ("Mortos Binds", 20, Some(false)),
            ]
        );
        assert_eq!(view.market_sales[0].credits, 1);
        assert_eq!(view.market_sales[2].platinum, 40);
    }

    #[test]
    fn fixture_market_sales() {
        let named: Vec<String> = view()
            .market_sales
            .into_iter()
            .map(|offer| offer.name)
            .collect();
        assert_eq!(
            named,
            vec![
                "Halloween 2023 Sentinel Bundle",
                "Dog Days Noggle Pack",
                "Quaro Armor Collection"
            ]
        );
        let world = WorldState::parse(FIXTURE).unwrap();
        let now = DateTime::from_timestamp_millis(FIXTURE_NOW_MS).unwrap();
        assert_eq!(world.market_sales(now).len(), 17);
    }

    #[test]
    fn baro_offers() {
        const CATALOG: &str = r#"[
            {"uniqueName":"/Lotus/Upgrades/Mods/Rifle/PrimeAmmoMutation","name":"Rifle Ammo Mutation",
             "category":"Mods","type":"Mod","tradable":true},
            {"uniqueName":"/Lotus/Weapons/Corpus/LongGuns/Prisma/PrismaGrakata","name":"Prisma Grakata",
             "category":"Primary","type":"Rifle","tradable":true},
            {"uniqueName":"/Lotus/Upgrades/Skins/Sigils/PrismaSigil","name":"Prisma Sigil",
             "category":"Skins","type":"Sigil","tradable":true},
            {"uniqueName":"/Lotus/Upgrades/Skins/Sigils/RhinoDeluxeSigil","name":"Rhino Palatine Sigil",
             "category":"Sigils","type":"Sigil","tradable":false},
            {"uniqueName":"/Lotus/Types/Items/MiscItems/PrimeBucks","name":"Ducats",
             "category":"Misc","type":"Misc","tradable":false}
        ]"#;
        const PRESENT: &str = r#"{"VoidTraders":[{
            "Activation":{"$date":{"$numberLong":"1788800000000"}},
            "Expiry":{"$date":{"$numberLong":"1788900000000"}},
            "Character":"Baro'Ki Teel",
            "Node":"TradeHUB1",
            "Manifest":[
              {"ItemType":"/Lotus/StoreItems/Weapons/Corpus/LongGuns/Prisma/PrismaGrakata","PrimePrice":550,"RegularPrice":250000},
              {"ItemType":"/Lotus/StoreItems/Upgrades/Mods/Rifle/PrimeAmmoMutation","PrimePrice":300,"RegularPrice":150000},
              {"ItemType":"/Lotus/StoreItems/Upgrades/Skins/Sigils/PrismaSigil","PrimePrice":200,"RegularPrice":100000},
              {"ItemType":"/Lotus/StoreItems/Types/Items/MiscItems/PrimeBucks","PrimePrice":1,"RegularPrice":1},
              {"ItemType":"/Lotus/StoreItems/Types/Boosters/AffinityBooster","PrimePrice":400,"RegularPrice":200000},
              {"ItemType":"/Lotus/StoreItems/Types/Game/Projections/T4VoidProjectionEBronze","PrimePrice":125,"RegularPrice":55000},
              {"ItemType":"/Lotus/StoreItems/Types/BoosterPacks/BaroTreasureBox","PrimePrice":0,"RegularPrice":50000},
              {"ItemType":"/Lotus/StoreItems/Types/Keys/MummyQuestKeyBlueprint","PrimePrice":100,"RegularPrice":25000},
              {"ItemType":"/Lotus/StoreItems/Upgrades/Skins/Sigils/RhinoDeluxeSigil","PrimePrice":45,"RegularPrice":55000}
            ]}]}"#;
        const RELICS: &str = r#"[{"uniqueName":"/Lotus/Types/Game/Projections/T4VoidProjectionEBronze",
            "name":"Axi A1 Intact","category":"Relics","type":"Relic","tradable":true,"rewards":[]}]"#;

        let world = WorldState::parse(PRESENT).expect("world state");
        let catalog = Catalog::from_json(CATALOG, RELICS, "[]").unwrap();
        let now = DateTime::from_timestamp_millis(1_788_850_000_000).unwrap();
        let view = WorldStateView::build(Some(&world), &catalog, None, now, Some(now));
        let groups: Vec<(&str, Vec<&str>)> = view
            .baro_manifest
            .iter()
            .map(|group| {
                (
                    group.name,
                    group
                        .items
                        .iter()
                        .map(|offer| offer.name.as_str())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            groups,
            vec![
                ("Mods", vec!["Rifle Ammo Mutation"]),
                ("Weapons", vec!["Prisma Grakata"]),
                ("Cosmetics", vec!["Prisma Sigil", "Rhino Palatine Sigil"]),
                (
                    "Others",
                    vec![
                        "Axi A1 Relic",
                        "Booster",
                        "Ducats",
                        "Sands of Inaros Blueprint"
                    ]
                ),
            ]
        );
        assert_eq!(view.baro_manifest[1].items[0].ducats, Some(550));
        assert_eq!(view.baro_manifest[1].items[0].credits, Some(250_000));
        assert_eq!(view.baro_manifest[1].items[0].owned, None);
    }

    #[test]
    fn baro_offers_owned() {
        const PRESENT: &str = r#"{"VoidTraders":[{
            "Activation":{"$date":{"$numberLong":"1788800000000"}},
            "Expiry":{"$date":{"$numberLong":"1788900000000"}},
            "Character":"Baro'Ki Teel",
            "Node":"TradeHUB1",
            "Manifest":[
              {"ItemType":"/Lotus/StoreItems/Upgrades/Skins/Sigils/BossSigilJackal","PrimePrice":50,"RegularPrice":50000},
              {"ItemType":"/Lotus/StoreItems/Upgrades/Skins/Sigils/PrismaSigil","PrimePrice":200,"RegularPrice":100000}
            ]}]}"#;

        let world = WorldState::parse(PRESENT).unwrap();
        let catalog = Catalog::from_json("[]", "[]", "[]").unwrap();
        let inventory = Inventory::parse(INVENTORY).unwrap();
        let now = DateTime::from_timestamp_millis(1_788_850_000_000).unwrap();
        let view = WorldStateView::build(Some(&world), &catalog, Some(&inventory), now, None);
        let owned: Vec<Option<bool>> = view.baro_manifest[0]
            .items
            .iter()
            .map(|offer| offer.owned)
            .collect();
        assert_eq!(owned, vec![Some(true), Some(false)]);
    }

    #[test]
    fn baro_away() {
        assert!(view().baro_manifest.is_empty());
    }

    #[test]
    fn nightwave_season() {
        let nightwave = view().nightwave.unwrap();
        assert_eq!(nightwave.season, 18);
        assert_eq!(nightwave.name, "Intermission 16");
        assert_eq!(nightwave.challenges.len(), 10);
    }

    #[test]
    fn empty_view_before_fetch() {
        let catalog = Catalog::from_json(ITEMS, "[]", "[]").unwrap();
        let now = DateTime::from_timestamp_millis(FIXTURE_NOW_MS).unwrap();
        let view = WorldStateView::build(None, &catalog, None, now, None);
        assert!(view.fissures.is_empty());
        assert!(view.timers.is_empty());
        assert!(view.daily_deals.is_empty());
        assert!(view.market_sales.is_empty());
        assert!(view.nightwave.is_none());
        assert!(view.fetched_at.is_none());
    }
}
