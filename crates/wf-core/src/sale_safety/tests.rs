use serde_json::{Value, json};
use wf_market::OrderType;

use super::*;
use crate::catalog::Catalog;
use crate::catalog::fixtures::{
    self, TRINITY_PRIME, TRINITY_PRIME_BLUEPRINT, TRINITY_PRIME_CHASSIS,
    TRINITY_PRIME_CHASSIS_BLUEPRINT, TRINITY_PRIME_HELMET_BLUEPRINT, TRINITY_PRIME_SYSTEMS,
    TRINITY_PRIME_SYSTEMS_BLUEPRINT, entries, market_item, push_counted,
};
use crate::identity::Variant;
use crate::inventory_view::tab;
use crate::prices::FixedPrices;
use crate::view::Fixture;

const BRONCO: &str = "/Lotus/Weapons/Tenno/Pistol/BroncoPrime";
const BRONCO_BLUEPRINT: &str = "/Lotus/Types/Recipes/Weapons/BroncoPrimeBlueprint";
const BRONCO_BARREL: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/BroncoPrimeBarrel";
const AKBRONCO: &str = "/Lotus/Weapons/Tenno/Akimbo/PrimeAkimboShotGun";
const AKBRONCO_BLUEPRINT: &str = "/Lotus/Types/Recipes/Weapons/AkbroncoPrimeBlueprint";
const AKBRONCO_LINK: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/AkbroncoPrimeLink";
const LATRON: &str = "/Lotus/Weapons/Tenno/Rifle/LatronWraith";
const LATRON_BLUEPRINT: &str = "/Lotus/Types/Recipes/Weapons/LatronWraithBlueprint";
const LATRON_BARREL: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/LatronWraithBarrel";

const UNMASTERED: SaleSafetyRule = keeping(SaleSafetyKeep::Unmastered);
const UNBUILT: SaleSafetyRule = keeping(SaleSafetyKeep::Unbuilt);
const NOTHING: SaleSafetyRule = keeping(SaleSafetyKeep::Nothing);

const fn keeping(keep: SaleSafetyKeep) -> SaleSafetyRule {
    SaleSafetyRule {
        keep,
        keep_favourites: false,
    }
}

fn weapons() -> Catalog {
    let weapon = |unique_name: &str, name: &str, parts: &[(&str, u32)]| {
        let parts: Vec<Value> = parts
            .iter()
            .map(|(part, count)| json!({ "uniqueName": part, "itemCount": count }))
            .collect();
        json!({
            "uniqueName": unique_name, "name": name, "category": "Secondary",
            "type": "Pistol", "tradable": false, "masterable": true, "components": parts,
        })
    };
    let items = json!([
        weapon(
            BRONCO,
            "Bronco Prime",
            &[(BRONCO_BLUEPRINT, 1), (BRONCO_BARREL, 1)]
        ),
        weapon(
            AKBRONCO,
            "Akbronco Prime",
            &[
                (AKBRONCO_BLUEPRINT, 1),
                (BRONCO, 1),
                (BRONCO, 1),
                (AKBRONCO_LINK, 2)
            ],
        ),
        weapon(
            LATRON,
            "Latron Wraith",
            &[(LATRON_BLUEPRINT, 1), (LATRON_BARREL, 1)]
        ),
    ]);
    let components: Vec<Value> = [
        (BRONCO_BLUEPRINT, "Blueprint"),
        (BRONCO_BARREL, "Barrel"),
        (AKBRONCO_BLUEPRINT, "Blueprint"),
        (AKBRONCO_LINK, "Link"),
        (LATRON_BLUEPRINT, "Blueprint"),
        (LATRON_BARREL, "Barrel"),
    ]
    .iter()
    .map(|(part, name)| json!({ "uniqueName": part, "name": name, "tradable": true }))
    .collect();
    Catalog::from_json(
        &items.to_string(),
        fixtures::RELICS,
        &Value::from(components).to_string(),
    )
    .unwrap()
}

fn holding(
    catalog: Catalog,
    built: &[&str],
    counted: &[(&str, i64)],
    mastered: &[&str],
    pending: &[&str],
) -> Fixture {
    let inventory = fixtures::inventory_with(|value| {
        for key in ["Suits", "Pistols", "MiscItems", "XPInfo", "PendingRecipes"] {
            value[key] = json!([]);
        }

        for item_type in built {
            entries(value, "Pistols").push(json!({
                "ItemType": item_type,
                "ItemId": { "$oid": "000000000000000000000001" },
            }));
        }

        for (item_type, count) in counted {
            push_counted(value, "MiscItems", item_type, *count);
        }

        for item_type in mastered {
            entries(value, "XPInfo").push(json!({ "ItemType": item_type, "XP": 10_000_000 }));
        }

        for item_type in pending {
            entries(value, "PendingRecipes").push(json!({
                "ItemType": item_type,
                "CompletionDate": { "$date": { "$numberLong": "1700000000000" } },
                "ItemId": { "$oid": "000000000000000000000002" },
            }));
        }
    });
    Fixture::new(catalog, inventory)
}

fn listing(fixture: Fixture, orders: &[(&str, &str, &str, u32)]) -> Fixture {
    let listed: Vec<wf_market::Item> = orders
        .iter()
        .map(|&(slug, game_ref, tag, _)| market_item(slug, slug, game_ref, &[tag]))
        .collect();
    let quantities = orders
        .iter()
        .map(|&(slug, _, _, quantity)| (slug, OrderType::Sell, quantity));
    Fixture {
        listings: MarketListings::new(&[]).with_orders(quantities),
        ..fixture
    }
    .with_market(&listed)
}

fn check(fixture: &Fixture, rule: SaleSafetyRule, record: &ItemRecord) -> MarketItemSaleSafety {
    SaleSafetyCheck::new(&fixture.view(), rule)
        .market_item(record)
        .unwrap()
}

fn part(fixture: &Fixture, rule: SaleSafetyRule, unique_name: &str) -> SaleSafety {
    check(fixture, rule, fixture.items.get(unique_name).unwrap()).safety
}

fn set(fixture: &Fixture, unique_name: &str) -> MarketItemSaleSafety {
    let record = fixture.items.variant(unique_name, Variant::Set).unwrap();
    check(fixture, NOTHING, record)
}

fn kept_for(item: &str, via: Option<&str>, reason: SaleSafetyReason) -> SaleSafetyReservation {
    SaleSafetyReservation {
        item: item.to_owned(),
        via: via.map(str::to_owned),
        reason,
    }
}

#[test]
fn unmastered_parent_reserves_its_parts() {
    let barrel = &[(BRONCO_BARREL, 1)];
    let unbuilt = holding(weapons(), &[], barrel, &[AKBRONCO], &[]);
    let safety = part(&unbuilt, UNMASTERED, BRONCO_BARREL);
    assert_eq!(safety.reserved, 1);
    assert_eq!(
        safety.reserved_for,
        [kept_for("Bronco Prime", None, SaleSafetyReason::Mastery)]
    );
    assert_eq!(safety.state, SaleSafetyState::NothingToList);

    let built = holding(weapons(), &[BRONCO], barrel, &[AKBRONCO], &[]);
    assert_eq!(part(&built, UNMASTERED, BRONCO_BARREL).reserved, 0);

    let sold = holding(weapons(), &[], barrel, &[AKBRONCO, BRONCO], &[]);
    assert_eq!(part(&sold, UNMASTERED, BRONCO_BARREL).reserved, 0);
}

#[test]
fn unbuilt_rule_keeps_parts_of_sold_items() {
    let sold = holding(
        weapons(),
        &[AKBRONCO],
        &[(BRONCO_BARREL, 1)],
        &[BRONCO],
        &[],
    );
    let safety = part(&sold, UNBUILT, BRONCO_BARREL);
    assert_eq!(safety.reserved, 1);
    assert_eq!(
        safety.reserved_for,
        [kept_for("Bronco Prime", None, SaleSafetyReason::Ownership)]
    );
}

#[test]
fn favourite_keeps_parts_until_built() {
    let favourites = SaleSafetyRule {
        keep_favourites: true,
        ..NOTHING
    };
    let barrel = &[(BRONCO_BARREL, 1)];
    let mut sold = holding(weapons(), &[], barrel, &[BRONCO], &[]);
    sold.favourites = [BRONCO].into_iter().collect();
    let safety = part(&sold, favourites, BRONCO_BARREL);
    assert_eq!(safety.reserved, 1);
    assert_eq!(
        safety.reserved_for,
        [kept_for("Bronco Prime", None, SaleSafetyReason::Favourite)]
    );
    assert_eq!(part(&sold, NOTHING, BRONCO_BARREL).reserved, 0);

    let mut built = holding(weapons(), &[BRONCO], barrel, &[], &[]);
    built.favourites = [BRONCO].into_iter().collect();
    assert_eq!(part(&built, favourites, BRONCO_BARREL).reserved, 0);
}

#[test]
fn recipe_count_reserves_every_copy() {
    let fixture = holding(weapons(), &[BRONCO], &[(AKBRONCO_LINK, 3)], &[BRONCO], &[])
        .with_prices(FixedPrices::new([("akbronco_prime_link", 10)]));
    let tab = tab(&fixture.view(), UNMASTERED);
    let link = &tab.parts[0];
    assert_eq!(link.sale.reserved, 2);
    assert_eq!(link.sale.state, SaleSafetyState::MoreToList { more: 1 });
    assert_eq!(link.listable_value, Some(10.0));
}

#[test]
fn pending_own_blueprint_frees_parts() {
    let chassis = &[(TRINITY_PRIME_CHASSIS_BLUEPRINT, 1)];
    let own = holding(
        fixtures::catalog(),
        &[],
        chassis,
        &[],
        &[TRINITY_PRIME_BLUEPRINT],
    );
    assert_eq!(part(&own, UNBUILT, TRINITY_PRIME_CHASSIS).reserved, 0);

    let systems = &[TRINITY_PRIME_SYSTEMS_BLUEPRINT];
    let systems = holding(fixtures::catalog(), &[], chassis, &[], systems);
    assert_eq!(part(&systems, UNBUILT, TRINITY_PRIME_CHASSIS).reserved, 1);
}

#[test]
fn pending_builds_cover_components_only() {
    let systems = holding(
        fixtures::catalog(),
        &[],
        &[(TRINITY_PRIME_SYSTEMS_BLUEPRINT, 2)],
        &[],
        &[TRINITY_PRIME_SYSTEMS_BLUEPRINT],
    );
    assert_eq!(part(&systems, UNBUILT, TRINITY_PRIME_SYSTEMS).reserved, 0);

    let bronco = holding(
        weapons(),
        &[],
        &[(BRONCO_BLUEPRINT, 2), (BRONCO_BARREL, 2)],
        &[],
        &[BRONCO_BLUEPRINT],
    );
    assert_eq!(part(&bronco, UNMASTERED, BRONCO_BARREL).reserved, 1);
    assert_eq!(part(&bronco, UNMASTERED, BRONCO_BLUEPRINT).reserved, 1);
}

#[test]
fn built_component_covers_its_blueprint() {
    let covered = holding(
        fixtures::catalog(),
        &[],
        &[
            (TRINITY_PRIME_SYSTEMS, 1),
            (TRINITY_PRIME_SYSTEMS_BLUEPRINT, 1),
        ],
        &[],
        &[],
    );
    assert_eq!(part(&covered, UNBUILT, TRINITY_PRIME_SYSTEMS).reserved, 0);

    let blueprint_only = holding(
        fixtures::catalog(),
        &[],
        &[(TRINITY_PRIME_SYSTEMS_BLUEPRINT, 1)],
        &[],
        &[],
    );
    assert_eq!(
        part(&blueprint_only, UNBUILT, TRINITY_PRIME_SYSTEMS).reserved,
        1
    );
}

#[test]
fn built_weapon_feeds_its_consumer() {
    let barrels = &[(BRONCO_BARREL, 3)];
    let unowned = holding(weapons(), &[], barrels, &[], &[]);
    let safety = part(&unowned, UNMASTERED, BRONCO_BARREL);
    assert_eq!(
        safety.reserved, 2,
        "master one Bronco, then build both into Akbronco"
    );
    assert_eq!(
        safety.reserved_for,
        [
            kept_for(
                "Akbronco Prime",
                Some("Bronco Prime"),
                SaleSafetyReason::Mastery
            ),
            kept_for("Bronco Prime", None, SaleSafetyReason::Mastery),
        ]
    );
    assert_eq!(part(&unowned, UNBUILT, BRONCO_BARREL).reserved, 3);

    let one_built = holding(weapons(), &[BRONCO], barrels, &[], &[]);
    let safety = part(&one_built, UNBUILT, BRONCO_BARREL);
    assert_eq!(safety.reserved, 2);
    assert_eq!(
        safety.reserved_for,
        [kept_for(
            "Akbronco Prime",
            Some("Bronco Prime"),
            SaleSafetyReason::Ownership
        )]
    );
}

#[test]
fn set_orders_count_against_each_part() {
    let orders = [
        ("akbronco_prime_link", AKBRONCO_LINK, "component", 1),
        ("akbronco_prime_set", AKBRONCO, "set", 1),
    ];
    let links = |count: i64| {
        let fixture = holding(weapons(), &[], &[(AKBRONCO_LINK, count)], &[], &[]);
        let fixture = listing(fixture, &orders);
        check(&fixture, NOTHING, fixture.items.get(AKBRONCO_LINK).unwrap())
    };
    let spare = links(4);
    assert_eq!(spare.safety.listed, Some(3));
    assert_eq!(spare.order_limit, 2);
    assert_eq!(spare.safety.state, SaleSafetyState::MoreToList { more: 1 });
    assert_eq!(links(3).safety.state, SaleSafetyState::AllListed);
    let over = links(2);
    assert_eq!(over.safety.state, SaleSafetyState::OverListed { over: 1 });
    assert_eq!(over.order_limit, 0);

    let signed_out = holding(weapons(), &[], &[(AKBRONCO_LINK, 4)], &[], &[]);
    let unknown = part(&signed_out, NOTHING, AKBRONCO_LINK);
    assert_eq!(unknown.listed, None);
    assert_eq!(unknown.state, SaleSafetyState::MoreToList { more: 4 });
}

#[test]
fn part_orders_count_as_listed_sets() {
    let sets = |count: i64| {
        let fixture = holding(
            weapons(),
            &[],
            &[(BRONCO_BLUEPRINT, count), (BRONCO_BARREL, count)],
            &[],
            &[],
        );
        let fixture = listing(
            fixture,
            &[("bronco_prime_barrel", BRONCO_BARREL, "component", 1)],
        );
        set(&fixture, BRONCO)
    };
    let two = sets(2);
    assert_eq!(two.owned, 2);
    assert_eq!(two.order_limit, 1);
    assert_eq!(two.safety.listed, Some(1));
    assert_eq!(two.safety.listed_as_parts, 1);
    assert_eq!(two.safety.state, SaleSafetyState::MoreToList { more: 1 });

    let one = sets(1);
    assert_eq!(one.order_limit, 0);
    assert_eq!(one.safety.state, SaleSafetyState::AllListed);
}

#[test]
fn sets_count_tradable_forms() {
    let counted = [
        (TRINITY_PRIME_BLUEPRINT, 1),
        (TRINITY_PRIME_CHASSIS_BLUEPRINT, 1),
        (TRINITY_PRIME_HELMET_BLUEPRINT, 1),
        (TRINITY_PRIME_SYSTEMS, 1),
    ];
    let fixture = holding(fixtures::catalog(), &[], &counted, &[], &[]);
    let fixture = listing(fixture, &[("trinity_prime_set", TRINITY_PRIME, "set", 1)]);
    let checked = set(&fixture, TRINITY_PRIME);
    assert_eq!(
        checked.owned, 0,
        "a built Systems is not a tradable set part"
    );
    assert_eq!(
        checked.safety.state,
        SaleSafetyState::OverListed { over: 1 }
    );

    let view = fixture.view();
    let record = fixture.items.variant(TRINITY_PRIME, Variant::Set).unwrap();
    assert_eq!(view.market_owned(record, &record.name, None, None), 0);
    assert!(tab(&view, NOTHING).sets.iter().all(|row| !row.complete));
}
