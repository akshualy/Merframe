use chrono::{DateTime, Utc};
use serde::Serialize;
use wf_data::{Item, Syndicate, SyndicateOffer, SyndicateTitle};
use wf_inventory::DailyStandingPool;
use wf_worldstate::ResetWindow;

use crate::catalog::is_part;
use crate::identity::{ItemKind, ItemRecord, market_slug};
use crate::reset_checklist::DailyStanding;
use crate::view::View;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SyndicateOfferRow {
    pub name: String,
    pub search_text: String,
    pub image_name: Option<String>,
    pub standing_cost: u32,
    pub required_rank: i32,
    pub available: bool,
    pub owned: bool,
    pub platinum: Option<u32>,
    pub platinum_per_thousand: Option<f64>,
    pub weekly_trades: Option<u32>,
    pub crafted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SyndicateRankUpItem {
    pub name: String,
    pub image_name: Option<String>,
    pub needed: u32,
    pub owned: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SyndicateRankUp {
    pub title: String,
    pub standing_short: i64,
    pub reached_before: bool,
    pub credits: u32,
    pub items: Vec<SyndicateRankUpItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SyndicateRankStanding {
    pub level: i32,
    pub top_level: i32,
    pub title: Option<String>,
    pub next: Option<SyndicateRankUp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SyndicateProgressMeter {
    pub earned: i64,
    pub span: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum SyndicateKind {
    Faction,
    Regular,
    Event,
    Conclave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum SyndicatePledgeRelation {
    Pledged,
    Allied,
    Opposed,
    Enemy,
}

fn pledge_relation(syndicate: &Syndicate, pledged: &str) -> Option<SyndicatePledgeRelation> {
    if syndicate.tag == pledged {
        return Some(SyndicatePledgeRelation::Pledged);
    }

    let alignment = *syndicate.alignments.get(pledged)?;
    Some(if alignment > 0.0 {
        SyndicatePledgeRelation::Allied
    } else if alignment > -1.0 {
        SyndicatePledgeRelation::Opposed
    } else {
        SyndicatePledgeRelation::Enemy
    })
}

fn standing_bin(syndicate: &Syndicate) -> (SyndicateKind, Option<DailyStandingPool>) {
    use DailyStandingPool as Pool;
    use SyndicateKind::{Conclave, Event, Faction, Regular};

    match syndicate.daily_limit_bin.as_str() {
        "STANDING_LIMIT_BIN_NORMAL" => (Faction, Some(Pool::Factions)),
        "STANDING_LIMIT_BIN_PVP" => (Conclave, Some(Pool::Pvp)),
        "STANDING_LIMIT_BIN_NONE" => (Event, None),
        "STANDING_LIMIT_BIN_CETUS" => (Regular, Some(Pool::Cetus)),
        "STANDING_LIMIT_BIN_QUILLS" => (Regular, Some(Pool::Quills)),
        "STANDING_LIMIT_BIN_SOLARIS" => (Regular, Some(Pool::Solaris)),
        "STANDING_LIMIT_BIN_VENTKIDS" => (Regular, Some(Pool::Ventkids)),
        "STANDING_LIMIT_BIN_VOX" => (Regular, Some(Pool::Vox)),
        "STANDING_LIMIT_BIN_ENTRATI" => (Regular, Some(Pool::Entrati)),
        "STANDING_LIMIT_BIN_NECRALOID" => (Regular, Some(Pool::Necraloid)),
        "STANDING_LIMIT_BIN_ZARIMAN" => (Regular, Some(Pool::Zariman)),
        "STANDING_LIMIT_BIN_CAVIA" => (Regular, Some(Pool::Cavia)),
        "STANDING_LIMIT_BIN_HEX" => (Regular, Some(Pool::Hex)),
        "STANDING_LIMIT_BIN_LIBRARY" => (Regular, Some(Pool::Library)),
        _ => (Regular, None),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SyndicateDailyStanding {
    pub left: u32,
    pub cap: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SyndicateOverview {
    pub tag: String,
    pub name: String,
    pub kind: SyndicateKind,
    pub pledge: Option<SyndicatePledgeRelation>,
    pub standing: Option<i64>,
    pub daily: Option<SyndicateDailyStanding>,
    pub meter: Option<SyndicateProgressMeter>,
    pub rank: Option<SyndicateRankStanding>,
    pub offers: Vec<SyndicateOfferRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SyndicatesTab {
    pub daily_reset: ResetWindow,
    pub faction_daily: Option<SyndicateDailyStanding>,
    pub syndicates: Vec<SyndicateOverview>,
}

const NEUTRAL_RANK: &str = "Neutral";
const SIMARIS_STANDING_CAP: i64 = 125_000;

fn sole_blueprint_product_slug(view: &View, record: &ItemRecord) -> Option<String> {
    let ItemKind::Part { set, .. } = record.kind else {
        return None;
    };
    let product = view.catalog.item(&view.items[set].unique_name)?;
    let parts = product
        .components
        .iter()
        .flatten()
        .filter(|component| is_part(component))
        .count();
    let sole_blueprint = record.name == format!("{} Blueprint", product.name) && parts <= 1;

    sole_blueprint.then(|| market_slug(&product.name))
}

fn offer_row(view: &View, offer: &SyndicateOffer, level: i32) -> SyndicateOfferRow {
    let record = view.items.resolve(&offer.store_item);
    let priced = |slug: String| view.prices.plat(&slug).map(|platinum| (slug, platinum));
    let own = record.market_slug.clone().and_then(priced);
    let product = own
        .is_none()
        .then(|| sole_blueprint_product_slug(view, &record))
        .flatten()
        .and_then(priced);
    let crafted = product.is_some();
    let (slug, platinum) = own.or(product).unzip();

    SyndicateOfferRow {
        platinum_per_thousand: platinum
            .filter(|_| offer.standing_cost > 0)
            .map(|platinum| f64::from(platinum) * 1000.0 / f64::from(offer.standing_cost)),
        platinum,
        weekly_trades: slug.and_then(|slug| view.prices.weekly_trades(&slug)),
        crafted,
        owned: view.account.owns(&record.unique_name),
        name: record.name.clone(),
        search_text: std::iter::once(record.name.clone())
            .chain(
                view.catalog
                    .item(&record.unique_name)
                    .into_iter()
                    .flat_map(Item::offer_search_terms),
            )
            .collect::<Vec<_>>()
            .join(" "),
        image_name: record.image_name.clone(),
        standing_cost: offer.standing_cost,
        required_rank: offer.required_level,
        available: offer.required_level <= level,
    }
}

struct SyndicateRankClimb<'a> {
    level: i32,
    title: &'a str,
    paid_with: &'a SyndicateTitle,
    from_standing: i64,
}

fn rank_climb<'a>(
    titles: &'a [SyndicateTitle],
    held: Option<&'a SyndicateTitle>,
    level: i32,
) -> Option<SyndicateRankClimb<'a>> {
    let above = titles.iter().find(|title| title.level == level + 1);

    if level >= 0 {
        return above.map(|title| SyndicateRankClimb {
            level: title.level,
            title: &title.name,
            paid_with: title,
            from_standing: title.min_standing,
        });
    }

    let held = held?;
    Some(SyndicateRankClimb {
        level: level + 1,
        title: above.map_or(NEUTRAL_RANK, |title| &title.name),
        paid_with: held,
        from_standing: held.max_standing,
    })
}

fn rank_up(
    view: &View,
    climb: &SyndicateRankClimb,
    standing: i64,
    reached: &[i32],
) -> SyndicateRankUp {
    let reached_before = reached.contains(&climb.level);
    let sacrifice = climb
        .paid_with
        .sacrifice
        .as_ref()
        .filter(|_| !reached_before);

    SyndicateRankUp {
        title: climb.title.to_owned(),
        standing_short: (climb.from_standing - standing).max(0),
        reached_before,
        credits: sacrifice.map_or_default(|sacrifice| sacrifice.credits),
        items: sacrifice
            .into_iter()
            .flat_map(|sacrifice| &sacrifice.items)
            .map(|item| {
                let record = view.items.resolve(&item.item_type);

                SyndicateRankUpItem {
                    name: record.name.clone(),
                    image_name: record.image_name.clone(),
                    needed: item.item_count,
                    owned: view.account.inventory.counted(&item.item_type),
                }
            })
            .collect(),
    }
}

fn progress_meter(
    titles: &[SyndicateTitle],
    held: Option<&SyndicateTitle>,
    level: i32,
    standing: Option<i64>,
) -> Option<SyndicateProgressMeter> {
    match (titles.last(), standing) {
        (Some(_), Some(standing)) => {
            let (floor, ceiling) = match held {
                Some(title) => (title.min_standing, title.max_standing),
                None => (
                    titles
                        .iter()
                        .rfind(|title| title.level < level)
                        .map_or_default(|title| title.max_standing),
                    titles
                        .iter()
                        .find(|title| title.level > level)
                        .map_or_default(|title| title.min_standing),
                ),
            };
            Some(SyndicateProgressMeter {
                earned: standing - floor,
                span: ceiling - floor,
            })
        }
        (Some(top), None) => Some(SyndicateProgressMeter {
            earned: i64::from(level),
            span: i64::from(top.level),
        }),
        (None, Some(standing)) => Some(SyndicateProgressMeter {
            earned: standing,
            span: SIMARIS_STANDING_CAP,
        }),
        (None, None) => None,
    }
}

fn daily_standing(
    daily: &DailyStanding,
    pool: DailyStandingPool,
) -> Option<SyndicateDailyStanding> {
    Some(SyndicateDailyStanding {
        left: daily.left(pool)?,
        cap: daily.cap,
    })
}

fn syndicate_overview(
    view: &View,
    syndicate: &Syndicate,
    daily: &DailyStanding,
) -> SyndicateOverview {
    let (kind, pool) = standing_bin(syndicate);
    let titles = &syndicate.titles;
    let affiliation = view
        .account
        .inventory
        .affiliations
        .iter()
        .find(|affiliation| affiliation.tag == syndicate.tag);
    let level = affiliation.map_or_default(|affiliation| affiliation.title);
    let held = titles.iter().find(|title| title.level == level);
    let reached =
        affiliation.map_or_default(|affiliation| affiliation.free_favors_earned.as_slice());
    let standing = (!syndicate.offers.is_empty())
        .then(|| affiliation.map_or_default(|affiliation| affiliation.standing));

    SyndicateOverview {
        tag: syndicate.tag.clone(),
        name: syndicate.name.clone(),
        kind,
        pledge: view
            .account
            .inventory
            .supported_syndicate
            .as_deref()
            .and_then(|pledged| pledge_relation(syndicate, pledged)),
        daily: pool
            .filter(|_| standing.is_some() && kind != SyndicateKind::Faction)
            .and_then(|pool| daily_standing(daily, pool)),
        standing,
        meter: progress_meter(titles, held, level, standing),
        rank: titles.last().map(|top| SyndicateRankStanding {
            level,
            top_level: top.level,
            title: held.map(|title| title.name.clone()),
            next: rank_climb(titles, held, level)
                .zip(standing)
                .map(|(climb, standing)| rank_up(view, &climb, standing, reached)),
        }),
        offers: syndicate
            .offers
            .iter()
            .map(|offer| offer_row(view, offer, level))
            .collect(),
    }
}

pub(crate) fn tab(view: &View, now: DateTime<Utc>) -> SyndicatesTab {
    let daily = DailyStanding::new(&view.account.inventory, now);
    let mut syndicates: Vec<_> = view
        .catalog
        .data()
        .syndicates()
        .iter()
        .map(|syndicate| syndicate_overview(view, syndicate, &daily))
        .collect();
    syndicates.sort_by(|a, b| {
        let level = |syndicate: &SyndicateOverview| {
            syndicate.rank.as_ref().map_or_default(|rank| rank.level)
        };
        level(b).cmp(&level(a)).then_with(|| a.name.cmp(&b.name))
    });

    SyndicatesTab {
        daily_reset: daily.reset,
        faction_daily: daily_standing(&daily, DailyStandingPool::Factions),
        syndicates,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use wf_data::{Dictionary, GameData, parse_syndicates};
    use wf_inventory::Inventory;

    use super::*;
    use crate::catalog::{Catalog, fixtures};
    use crate::prices::FixedPrices;
    use crate::reset_checklist::fixtures::synced_at;
    use crate::view::Fixture;

    const STEEL_MERIDIAN: &str = "SteelMeridianSyndicate";

    fn tab_of(inventory: Inventory) -> SyndicatesTab {
        tab(
            &Fixture::new(fixtures::syndicate_catalog(), inventory).view(),
            synced_at(),
        )
    }

    fn affiliated(tag: &str, affiliation: Option<Value>) -> Inventory {
        fixtures::inventory_with(|value| {
            let affiliations = fixtures::entries(value, "Affiliations");
            affiliations.retain(|entry| entry["Tag"] != tag);
            affiliations.extend(affiliation);
        })
    }

    fn syndicate(tab: SyndicatesTab, tag: &str) -> SyndicateOverview {
        tab.syndicates
            .into_iter()
            .find(|syndicate| syndicate.tag == tag)
            .unwrap()
    }

    fn steel_meridian(tab: SyndicatesTab) -> SyndicateOverview {
        syndicate(tab, STEEL_MERIDIAN)
    }

    fn steel_meridian_at(title: i32, standing: i64) -> SyndicateOverview {
        steel_meridian(tab_of(affiliated(
            STEEL_MERIDIAN,
            Some(json!({ "Tag": STEEL_MERIDIAN, "Title": title, "Standing": standing })),
        )))
    }

    #[test]
    fn rank_with_next_sacrifice() {
        let brave = steel_meridian_at(1, 12_000);

        assert_eq!(brave.standing, Some(12_000));
        assert_eq!(
            brave.meter,
            Some(SyndicateProgressMeter {
                earned: 7_000,
                span: 22_000,
            })
        );
        assert_eq!(
            brave.rank,
            Some(SyndicateRankStanding {
                level: 1,
                top_level: 5,
                title: Some("Brave".to_owned()),
                next: Some(SyndicateRankUp {
                    title: "Valiant".to_owned(),
                    standing_short: 15_000,
                    reached_before: false,
                    credits: 50_000,
                    items: vec![SyndicateRankUpItem {
                        name: "Forma".to_owned(),
                        image_name: Some("GenericComponent.png".to_owned()),
                        needed: 1,
                        owned: 65,
                    }],
                }),
            })
        );
    }

    #[test]
    fn neutral_rank_between_titles() {
        let neutral = steel_meridian(tab_of(affiliated(STEEL_MERIDIAN, None)));
        let below_zero = steel_meridian_at(0, -3_000);
        let rank = neutral.rank.unwrap();

        assert_eq!(neutral.standing, Some(0));
        assert_eq!((rank.level, rank.title), (0, None));
        assert_eq!(
            neutral.meter,
            Some(SyndicateProgressMeter {
                earned: 5_000,
                span: 10_000,
            })
        );
        assert_eq!(
            below_zero.meter,
            Some(SyndicateProgressMeter {
                earned: 2_000,
                span: 10_000,
            })
        );
        assert_eq!(rank.next.unwrap().title, "Brave");
    }

    #[test]
    fn no_rank_up_at_the_top() {
        let general = steel_meridian_at(5, 372_000);

        assert_eq!(
            general.meter,
            Some(SyndicateProgressMeter {
                earned: 132_000,
                span: 132_000,
            })
        );
        assert_eq!(general.rank.unwrap().next, None);
    }

    #[test]
    fn negative_rank_climbs_with_its_own_sacrifice() {
        let enemy = steel_meridian_at(-2, -71_000);
        let meter = enemy.meter.unwrap();
        let enemy = enemy.rank.unwrap();
        let outcast = steel_meridian_at(-1, -6_000).rank.unwrap();
        let to_outcast = enemy.next.unwrap();
        let to_neutral = outcast.next.unwrap();

        assert_eq!(enemy.title.as_deref(), Some("Enemy"));
        assert_eq!((meter.earned, meter.span), (0, 44_000));
        assert_eq!(
            (to_outcast.title.as_str(), to_outcast.standing_short),
            ("Outcast", 44_000)
        );
        assert_eq!(to_outcast.items[0].name, "Orokin Catalyst");
        assert_eq!(
            (to_neutral.title.as_str(), to_neutral.standing_short),
            ("Neutral", 1_000)
        );
        assert_eq!(to_neutral.items[0].name, "Forma");
    }

    #[test]
    fn standing_only_where_offers_cost_it() {
        let simaris = syndicate(tab_of(fixtures::inventory()), "LibrarySyndicate");
        let garrison = syndicate(
            tab_of(affiliated(
                "KahlSyndicate",
                Some(json!({ "Tag": "KahlSyndicate", "Title": 3, "Standing": 3 })),
            )),
            "KahlSyndicate",
        );

        assert_eq!(simaris.standing, Some(74_639));
        assert_eq!(simaris.rank, None);
        assert_eq!(
            simaris.meter,
            Some(SyndicateProgressMeter {
                earned: 74_639,
                span: 125_000,
            })
        );
        assert_eq!(garrison.standing, None);
        assert_eq!(
            garrison.meter,
            Some(SyndicateProgressMeter { earned: 3, span: 5 })
        );
        assert_eq!(
            garrison
                .rank
                .map(|rank| (rank.level, rank.title, rank.next)),
            Some((3, Some("Fort".to_owned()), None))
        );
    }

    #[test]
    fn offers_priced_and_owned() {
        let fixture = Fixture::new(fixtures::syndicate_catalog(), fixtures::inventory())
            .with_prices(
                FixedPrices::new([("justice_blades", 15)])
                    .with_weekly_trades([("justice_blades", 42)]),
            );
        let offers = steel_meridian(tab(&fixture.view(), synced_at())).offers;
        let blades = offers
            .iter()
            .find(|offer| offer.name == "Justice Blades")
            .unwrap();
        let pack = offers
            .iter()
            .find(|offer| offer.name == "Random Syndicate Projection Pack")
            .unwrap();

        assert_eq!(
            *blades,
            SyndicateOfferRow {
                name: "Justice Blades".to_owned(),
                search_text: "Justice Blades".to_owned(),
                image_name: Some("DualCleavers.jpg".to_owned()),
                standing_cost: 25_000,
                required_rank: 4,
                available: true,
                owned: true,
                platinum: Some(15),
                platinum_per_thousand: Some(0.6),
                weekly_trades: Some(42),
                crafted: false,
            }
        );
        assert!(!pack.owned);
        assert_eq!(pack.platinum, None);
    }

    #[test]
    fn offers_available_from_their_rank() {
        let available = |title| {
            steel_meridian_at(title, 0)
                .offers
                .into_iter()
                .filter(|offer| offer.available)
                .map(|offer| offer.name)
                .collect::<Vec<_>>()
        };

        assert_eq!(available(3), ["Random Syndicate Projection Pack"]);
        assert_eq!(
            available(4),
            ["Random Syndicate Projection Pack", "Justice Blades"]
        );
    }

    const LENS_BLUEPRINT: &str = "/Lotus/StoreItems/Types/Recipes/Lens/PowerLensOstronBlueprint";

    fn event_fixture(items: &str, store_item: &str, prices: FixedPrices) -> Fixture {
        let dictionary = Dictionary::from_json(
            r#"{"/Lotus/Language/Syndicates/EventName":"Operational Supply"}"#,
        )
        .unwrap();
        let export = json!({ "EventSyndicate": {
            "name": "/Lotus/Language/Syndicates/EventName",
            "dailyLimitBin": "STANDING_LIMIT_BIN_NONE",
            "favours": [{
                "storeItem": store_item,
                "standingCost": 5000,
                "requiredLevel": 0,
            }],
        }});
        let data = GameData::from_json(items, fixtures::RELICS, fixtures::COMPONENTS).unwrap();
        let syndicates = parse_syndicates(&export.to_string(), &dictionary).unwrap();

        Fixture::new(
            Catalog::new(data.with_syndicates(syndicates)),
            fixtures::inventory(),
        )
        .with_prices(prices)
    }

    fn lens_fixture(prices: FixedPrices) -> Fixture {
        event_fixture(fixtures::FOUNDRY_ITEMS, LENS_BLUEPRINT, prices)
    }

    fn only_offer(fixture: &Fixture) -> SyndicateOfferRow {
        tab(&fixture.view(), synced_at())
            .syndicates
            .remove(0)
            .offers
            .remove(0)
    }

    #[test]
    fn sole_blueprint_priced_as_its_product() {
        let crafted = only_offer(&lens_fixture(
            FixedPrices::new([("eidolon_zenurik_lens", 40)])
                .with_weekly_trades([("eidolon_zenurik_lens", 20)]),
        ));
        let traded = only_offer(&lens_fixture(FixedPrices::new([
            ("eidolon_zenurik_lens", 40),
            ("eidolon_zenurik_lens_blueprint", 12),
        ])));

        assert_eq!(crafted.name, "Eidolon Zenurik Lens Blueprint");
        assert!(crafted.crafted);
        assert_eq!(crafted.platinum, Some(40));
        assert_eq!(crafted.platinum_per_thousand, Some(8.0));
        assert_eq!(crafted.weekly_trades, Some(20));
        assert!(!traded.crafted);
        assert_eq!(traded.platinum, Some(12));
    }

    #[test]
    fn blueprint_with_other_parts_stays_unpriced() {
        let store_item = fixtures::BRATON_PRIME_BLUEPRINT.replace("/Lotus/", "/Lotus/StoreItems/");
        let prices = FixedPrices::new([("braton_prime", 90), ("braton_prime_set", 90)]);
        let blueprint = only_offer(&event_fixture(fixtures::ITEMS, &store_item, prices));

        assert_eq!(blueprint.name, "Braton Prime Blueprint");
        assert!(!blueprint.crafted);
        assert_eq!(blueprint.platinum, None);
    }

    #[test]
    fn rank_reached_before_needs_no_sacrifice() {
        let back_to_valiant = steel_meridian(tab_of(affiliated(
            STEEL_MERIDIAN,
            Some(json!({
                "Tag": STEEL_MERIDIAN,
                "Title": 1,
                "Standing": 12_000,
                "FreeFavorsEarned": [1, 2, 3],
            })),
        )))
        .rank
        .unwrap()
        .next
        .unwrap();

        assert!(back_to_valiant.reached_before);
        assert_eq!(back_to_valiant.credits, 0);
        assert!(
            back_to_valiant.items.is_empty(),
            "{:?}",
            back_to_valiant.items
        );
    }

    #[test]
    fn highest_rank_first_then_by_name() {
        let inventory = affiliated(
            STEEL_MERIDIAN,
            Some(json!({ "Tag": STEEL_MERIDIAN, "Title": 1, "Standing": 12_000 })),
        );
        let names: Vec<_> = tab_of(inventory)
            .syndicates
            .into_iter()
            .map(|syndicate| syndicate.name)
            .collect();

        assert_eq!(
            names,
            ["Garrison", "Ventkids", "Steel Meridian", "Cephalon Simaris"]
        );
    }

    #[test]
    fn relation_to_the_pledged_syndicate() {
        let relation = |pledged: Option<&str>| {
            let inventory =
                fixtures::inventory_with(|value| value["SupportedSyndicate"] = json!(pledged));
            steel_meridian(tab_of(inventory)).pledge
        };

        assert_eq!(relation(None), None);
        assert_eq!(
            relation(Some(STEEL_MERIDIAN)),
            Some(SyndicatePledgeRelation::Pledged)
        );
        assert_eq!(
            relation(Some("RedVeilSyndicate")),
            Some(SyndicatePledgeRelation::Allied)
        );
        assert_eq!(
            relation(Some("NewLokaSyndicate")),
            Some(SyndicatePledgeRelation::Opposed)
        );
        assert_eq!(
            relation(Some("PerrinSyndicate")),
            Some(SyndicatePledgeRelation::Enemy)
        );
        assert_eq!(relation(Some("CetusSyndicate")), None);
    }

    #[test]
    fn standing_left_today() {
        let today = tab_of(fixtures::inventory());
        let shared = Some(SyndicateDailyStanding {
            left: 21_000,
            cap: 23_000,
        });

        assert_eq!(today.faction_daily, shared);
        assert_eq!(steel_meridian(today.clone()).daily, None);
        assert_eq!(
            syndicate(today.clone(), "LibrarySyndicate").daily,
            Some(SyndicateDailyStanding {
                left: 0,
                cap: 23_000,
            })
        );
        assert_eq!(syndicate(today, "KahlSyndicate").daily, None);
    }
}
