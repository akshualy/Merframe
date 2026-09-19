use std::cmp::Reverse;
use std::collections::HashSet;

use wf_data::{Component, Drop, Item, Refinement};

use crate::catalog::{Catalog, REFINEMENTS, Stock, is_part, part_market_slug, refinement_name};
use crate::prices::{PriceSource, market_slug};
use crate::relic_planner::owned_relic_count;

use super::{NodeDrop, NodeMarket, OwnedRelic};

const WIKI: &str = "https://wiki.warframe.com/w/";
const OWNED_RELIC_ROWS: usize = 5;

pub(super) fn wiki_url(catalog: &Catalog, unique_name: &str) -> Option<String> {
    let item = catalog.item(unique_name)?;
    Some(match &item.wikia_url {
        Some(url) => url.replace("https://warframe.fandom.com/wiki/", WIKI),
        None => format!("{WIKI}{}", item.name.replace(' ', "_")),
    })
}

fn named_refinement(name: &str) -> Option<Refinement> {
    REFINEMENTS
        .into_iter()
        .find(|refinement| refinement_name(*refinement) == name)
}

fn relic_refinement(location: &str) -> Option<Refinement> {
    let (relic, refinement) = match location
        .strip_suffix(')')
        .and_then(|head| head.rsplit_once(" ("))
    {
        Some((head, name)) => (head, named_refinement(name)?),
        None => (location, Refinement::Intact),
    };
    relic.ends_with(" Relic").then_some(refinement)
}

fn owned_of(row: &NodeDrop) -> i64 {
    match row {
        NodeDrop::Relic { owned, .. } => *owned,
        NodeDrop::Purchase { .. } | NodeDrop::Location { .. } => 0,
    }
}

fn chance_of(row: &NodeDrop) -> f64 {
    match row {
        NodeDrop::Relic { chance, .. } | NodeDrop::Location { chance, .. } => *chance,
        NodeDrop::Purchase { .. } => 0.0,
    }
}

pub(super) fn node_drops(catalog: &Catalog, stock: &Stock<'_>, drops: &[Drop]) -> Vec<NodeDrop> {
    let mut listed: HashSet<&str> = HashSet::new();
    let mut rows: Vec<NodeDrop> = Vec::new();
    for drop in drops {
        match relic_refinement(&drop.location) {
            Some(Refinement::Intact) => {
                let Some(unique_name) = drop.unique_name.as_deref() else {
                    continue;
                };
                let Some((relic, _)) = catalog.relic_by_unique_name(unique_name) else {
                    continue;
                };
                if !listed.insert(unique_name) {
                    continue;
                }
                rows.push(NodeDrop::Relic {
                    unique_name: unique_name.to_owned(),
                    name: relic.name.clone(),
                    image_name: relic.image_name.clone(),
                    owned: owned_relic_count(stock, relic),
                    chance: drop.chance,
                    vaulted: relic.vaulted,
                });
            }
            Some(_) => {}
            None => rows.push(NodeDrop::Location {
                location: drop.location.clone(),
                chance: drop.chance,
            }),
        }
    }
    rows.sort_by(|left, right| {
        owned_of(right)
            .cmp(&owned_of(left))
            .then_with(|| chance_of(right).total_cmp(&chance_of(left)))
    });
    rows
}

pub(super) fn component_drops(
    catalog: &Catalog,
    stock: &Stock<'_>,
    parent: &Item,
    component: &Component,
) -> Vec<NodeDrop> {
    let mut rows = node_drops(
        catalog,
        stock,
        component.drops.as_deref().unwrap_or_default(),
    );
    if let Some(credits) = parent.blueprint_cost
        && rows.is_empty()
        && component.name == "Blueprint"
    {
        rows.push(NodeDrop::Purchase { credits });
    }
    rows
}

pub(super) fn node_market(
    prices: &dyn PriceSource,
    parent: &Item,
    component: &Component,
) -> Option<NodeMarket> {
    if !component.tradable {
        return None;
    }
    let slug = if is_part(component) {
        part_market_slug(parent, component)
    } else {
        market_slug(&component.name)
    };
    let sell = prices.plat(&slug)?;
    Some(NodeMarket { slug, sell })
}

pub(super) fn owned_relics(
    catalog: &Catalog,
    stock: &Stock<'_>,
    drops: &[Drop],
) -> Vec<OwnedRelic> {
    let mut listed: HashSet<&str> = HashSet::new();
    let mut rows: Vec<OwnedRelic> = Vec::new();
    for drop in drops {
        let Some(refinement) = relic_refinement(&drop.location) else {
            continue;
        };
        let Some((relic, _)) = drop
            .unique_name
            .as_deref()
            .and_then(|unique_name| catalog.relic_by_unique_name(unique_name))
        else {
            continue;
        };
        let Some(unique_name) = relic.unique_names.get(&refinement) else {
            continue;
        };
        let owned = stock.count(unique_name);
        if owned == 0 || !listed.insert(unique_name.as_str()) {
            continue;
        }
        rows.push(OwnedRelic {
            unique_name: unique_name.clone(),
            name: format!("{} {}", relic.name, refinement_name(refinement)),
            image_name: relic.image_names.get(&refinement).cloned(),
            owned,
            chance: drop.chance,
        });
    }
    rows.sort_by_key(|row| Reverse(row.owned));
    rows.truncate(OWNED_RELIC_ROWS);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::prices::FixedPrices;
    use wf_data::Rarity;
    use wf_inventory::Inventory;

    const BRATON_PRIME_STOCK: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock";
    const EXCALIBUR: &str = "/Lotus/Powersuits/Excalibur/Excalibur";
    const FERRITE: &str = "/Lotus/Types/Items/MiscItems/Ferrite";
    const OROKIN_CELL: &str = "/Lotus/Types/Items/MiscItems/OrokinCell";
    const AXI_A1_INTACT: &str = "/Lotus/Types/Game/Projections/T4VoidProjectionEBronze";
    const AXI_A1_EXCEPTIONAL: &str = "/Lotus/Types/Game/Projections/T4VoidProjectionESilver";
    const AXI_A1_FLAWLESS: &str = "/Lotus/Types/Game/Projections/T4VoidProjectionEGold";
    const AXI_A1_RADIANT: &str = "/Lotus/Types/Game/Projections/T4VoidProjectionEPlatinum";
    const AXI_A21_INTACT: &str =
        "/Lotus/Types/Game/Projections/T4VoidProjectionStyanaxPrimeABronze";
    const AXI_A21_EXCEPTIONAL: &str =
        "/Lotus/Types/Game/Projections/T4VoidProjectionStyanaxPrimeASilver";
    const AXI_A21_FLAWLESS: &str =
        "/Lotus/Types/Game/Projections/T4VoidProjectionStyanaxPrimeAGold";
    const AXI_A21_RADIANT: &str =
        "/Lotus/Types/Game/Projections/T4VoidProjectionStyanaxPrimeAPlatinum";

    fn listed_drops<'a>(catalog: &'a Catalog, unique_name: &str) -> &'a [Drop] {
        let (_, component) = catalog.component(unique_name).unwrap();
        component.drops.as_deref().unwrap()
    }

    fn drop_row(location: &str, chance: f64, unique_name: Option<&str>) -> Drop {
        Drop {
            chance,
            location: location.to_owned(),
            rarity: Rarity::Uncommon,
            drop_type: "Braton Prime Stock".to_owned(),
            unique_name: unique_name.map(str::to_owned),
        }
    }

    fn relic_stock(counted: &[(&str, i64)]) -> Inventory {
        let owned: Vec<(&str, &str, i64)> = counted
            .iter()
            .map(|(unique_name, count)| ("MiscItems", *unique_name, *count))
            .collect();
        fixtures::inventory_stocked(&[], &owned)
    }

    #[test]
    fn wiki_link_prefix_and_resource_names() {
        let catalog = fixtures::catalog();
        assert_eq!(
            wiki_url(&catalog, EXCALIBUR).as_deref(),
            Some("https://wiki.warframe.com/w/Excalibur")
        );
        assert_eq!(wiki_url(&catalog, "/Lotus/Nope"), None);

        let fandom = fixtures::ITEMS.replace(WIKI, "https://warframe.fandom.com/wiki/");
        let moved = Catalog::from_json(&fandom, fixtures::RELICS).unwrap();
        assert_eq!(
            wiki_url(&moved, EXCALIBUR).as_deref(),
            Some("https://wiki.warframe.com/w/Excalibur")
        );

        let resources = fixtures::foundry_catalog();
        assert_eq!(
            wiki_url(&resources, FERRITE).as_deref(),
            Some("https://wiki.warframe.com/w/Ferrite")
        );
        assert_eq!(
            wiki_url(&resources, OROKIN_CELL).as_deref(),
            Some("https://wiki.warframe.com/w/Orokin_Cell")
        );
    }

    #[test]
    fn refinements_collapse_into_one_relic_row() {
        let catalog = fixtures::catalog();
        let inventory = relic_stock(&[(AXI_A1_INTACT, 4), (AXI_A1_RADIANT, 7)]);
        let stock = Stock::new(&inventory);
        let rows = node_drops(&catalog, &stock, listed_drops(&catalog, BRATON_PRIME_STOCK));
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            NodeDrop::Relic {
                unique_name: AXI_A1_INTACT.to_owned(),
                name: "Axi A1".to_owned(),
                image_name: Some("RelicAxiA.png".to_owned()),
                owned: 11,
                chance: 25.33,
                vaulted: true,
            }
        );
    }

    #[test]
    fn rows_read_owned_first_then_chance() {
        let catalog = fixtures::catalog();
        let inventory = relic_stock(&[(AXI_A21_INTACT, 3), (AXI_A21_RADIANT, 19)]);
        let stock = Stock::new(&inventory);
        let rows = node_drops(
            &catalog,
            &stock,
            &[
                drop_row("Earth/Everest (Excavation), Rotation C", 10.0, None),
                drop_row("Axi A1 Relic", 25.33, Some(AXI_A1_INTACT)),
                drop_row("Axi A21 Relic", 11.0, Some(AXI_A21_INTACT)),
                drop_row("Axi A21 Relic (Radiant)", 16.67, Some(AXI_A21_INTACT)),
                drop_row(
                    "Lith Z9 Relic",
                    30.0,
                    Some("/Lotus/Types/Game/Projections/T1Nope"),
                ),
            ],
        );
        assert_eq!(
            rows,
            vec![
                NodeDrop::Relic {
                    unique_name: AXI_A21_INTACT.to_owned(),
                    name: "Axi A21".to_owned(),
                    image_name: Some("RelicAxiA.png".to_owned()),
                    owned: 22,
                    chance: 11.0,
                    vaulted: false,
                },
                NodeDrop::Relic {
                    unique_name: AXI_A1_INTACT.to_owned(),
                    name: "Axi A1".to_owned(),
                    image_name: Some("RelicAxiA.png".to_owned()),
                    owned: 0,
                    chance: 25.33,
                    vaulted: true,
                },
                NodeDrop::Location {
                    location: "Earth/Everest (Excavation), Rotation C".to_owned(),
                    chance: 10.0,
                },
            ]
        );
    }

    #[test]
    fn owned_relics_keep_the_five_fullest() {
        let catalog = fixtures::catalog();
        let inventory = relic_stock(&[
            (AXI_A1_EXCEPTIONAL, 2),
            (AXI_A1_FLAWLESS, 9),
            (AXI_A1_RADIANT, 4),
            (AXI_A21_INTACT, 6),
            (AXI_A21_EXCEPTIONAL, 1),
            (AXI_A21_FLAWLESS, 12),
            (AXI_A21_RADIANT, 5),
        ]);
        let stock = Stock::new(&inventory);
        let drops: Vec<Drop> = [
            ("Axi A1 Relic", AXI_A1_INTACT),
            ("Axi A1 Relic (Exceptional)", AXI_A1_INTACT),
            ("Axi A1 Relic (Flawless)", AXI_A1_INTACT),
            ("Axi A1 Relic (Radiant)", AXI_A1_INTACT),
            ("Axi A21 Relic", AXI_A21_INTACT),
            ("Axi A21 Relic (Exceptional)", AXI_A21_INTACT),
            ("Axi A21 Relic (Flawless)", AXI_A21_INTACT),
            ("Axi A21 Relic (Radiant)", AXI_A21_INTACT),
        ]
        .into_iter()
        .map(|(location, unique_name)| drop_row(location, 11.0, Some(unique_name)))
        .collect();

        let rows = owned_relics(&catalog, &stock, &drops);
        assert_eq!(
            rows.iter()
                .map(|row| (row.name.as_str(), row.owned))
                .collect::<Vec<(&str, i64)>>(),
            vec![
                ("Axi A21 Flawless", 12),
                ("Axi A1 Flawless", 9),
                ("Axi A21 Intact", 6),
                ("Axi A21 Radiant", 5),
                ("Axi A1 Radiant", 4),
            ]
        );
        assert!(rows.iter().all(|row| row.image_name.is_some()));
        assert_eq!(
            rows[0].unique_name.as_str(),
            "/Lotus/Types/Game/Projections/T4VoidProjectionStyanaxPrimeAGold"
        );
    }

    #[test]
    fn owned_relics_skip_what_the_account_lacks() {
        let catalog = fixtures::catalog();
        let inventory = relic_stock(&[(AXI_A1_FLAWLESS, 3)]);
        let stock = Stock::new(&inventory);
        let rows = owned_relics(&catalog, &stock, listed_drops(&catalog, BRATON_PRIME_STOCK));
        assert_eq!(
            rows.iter()
                .map(|row| (row.name.as_str(), row.owned))
                .collect::<Vec<(&str, i64)>>(),
            vec![("Axi A1 Flawless", 3)]
        );

        let empty = relic_stock(&[]);
        assert!(
            owned_relics(
                &catalog,
                &Stock::new(&empty),
                listed_drops(&catalog, BRATON_PRIME_STOCK)
            )
            .is_empty()
        );
    }

    #[test]
    fn a_blueprint_without_drops_is_bought_with_credits() {
        let catalog = fixtures::catalog();
        let inventory = relic_stock(&[]);
        let stock = Stock::new(&inventory);
        let excalibur = catalog.item(EXCALIBUR).unwrap();
        let components = excalibur.components.as_deref().unwrap();
        let blueprint = components
            .iter()
            .find(|component| component.name == "Blueprint")
            .unwrap();
        assert_eq!(
            component_drops(&catalog, &stock, excalibur, blueprint),
            vec![NodeDrop::Purchase { credits: 35_000 }]
        );
        let chassis = components
            .iter()
            .find(|component| component.name == "Chassis")
            .unwrap();
        assert!(
            component_drops(&catalog, &stock, excalibur, chassis)
                .iter()
                .all(|row| matches!(row, NodeDrop::Location { .. }))
        );
    }

    #[test]
    fn only_tradable_priced_parts_carry_a_market_quote() {
        let catalog = fixtures::catalog();
        let (braton_prime, stock_part) = catalog.component(BRATON_PRIME_STOCK).unwrap();
        let priced = FixedPrices::new([("braton_prime_stock", 12.0)]);
        assert_eq!(
            node_market(&priced, braton_prime, stock_part),
            Some(NodeMarket {
                slug: "braton_prime_stock".to_owned(),
                sell: 12.0,
            })
        );
        assert_eq!(
            node_market(&FixedPrices::default(), braton_prime, stock_part),
            None
        );

        let excalibur = catalog.item(EXCALIBUR).unwrap();
        let chassis = &excalibur.components.as_deref().unwrap()[1];
        let everything = FixedPrices::new([("excalibur_chassis_blueprint", 5.0)]);
        assert_eq!(node_market(&everything, excalibur, chassis), None);
    }
}
