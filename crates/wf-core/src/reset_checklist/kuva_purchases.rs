use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, TimeDelta, Utc};
use wf_inventory::Inventory;
use wf_worldstate::game_week_count;

use crate::catalog::KUVA_ITEM;
use crate::reset_checklist::cadence::ResetCadence;
use crate::reset_checklist::vendors::{
    TESHIN_MANIFEST, VENDORS, VendorKuvaCertainty, VendorNamedOffer, purchases_expiring,
    weekly_purchases,
};

const TESHIN_ROTATION_WEEKS: i64 = 8;
const TESHIN_ROTATION_KUVA_WEEK: i64 = 666;
const TESHIN_ROTATION_KUVA: i64 = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VendorKuvaBought {
    pub(crate) manifest: &'static str,
    pub(crate) bought: u32,
}

struct EstimatedVendorPurchases {
    manifest: &'static str,
    amount: u32,
    bought_counts: BTreeSet<u32>,
}

pub(super) fn kuva_bought_between(
    previous: &Inventory,
    next: &Inventory,
    now: DateTime<Utc>,
) -> Vec<VendorKuvaBought> {
    let mut unexplained = next.counted(KUVA_ITEM) - previous.counted(KUVA_ITEM);
    let mut estimated = Vec::new();

    if teshin_rotation_kuva_bought(next, now) && !teshin_rotation_kuva_bought(previous, now) {
        unexplained -= TESHIN_ROTATION_KUVA;
    }

    for vendor in &VENDORS {
        let Some(VendorNamedOffer::Kuva {
            amount,
            limit,
            certainty,
            ..
        }) = vendor.offer
        else {
            continue;
        };

        let before: HashMap<&str, u32> = weekly_purchases(previous, vendor.manifest, now)
            .map(|purchase| (purchase.item_id.as_str(), purchase.num_purchased))
            .collect();
        let bought = weekly_purchases(next, vendor.manifest, now)
            .map(|purchase| {
                let already = before.get(purchase.item_id.as_str()).copied().unwrap_or(0);
                purchase.num_purchased.saturating_sub(already)
            })
            .filter(|&bought| bought > 0);
        match certainty {
            VendorKuvaCertainty::Confirmed => {
                unexplained -= i64::from(amount) * i64::from(bought.sum::<u32>());
            }
            VendorKuvaCertainty::Estimated => estimated.push(EstimatedVendorPurchases {
                manifest: vendor.manifest,
                amount,
                bought_counts: bought.filter(|&bought| bought <= limit).collect(),
            }),
        }
    }

    let mut choices = kuva_purchase_choices(&estimated, unexplained).into_iter();
    match (choices.next(), choices.next()) {
        (Some(only), None) => only,
        _ => Vec::new(),
    }
}

fn teshin_rotation_kuva_bought(inventory: &Inventory, now: DateTime<Utc>) -> bool {
    let offered_again =
        ResetCadence::Weekly.window(now).activation + TimeDelta::weeks(TESHIN_ROTATION_WEEKS);

    (game_week_count(now) - TESHIN_ROTATION_KUVA_WEEK).rem_euclid(TESHIN_ROTATION_WEEKS) == 0
        && purchases_expiring(inventory, TESHIN_MANIFEST, now, offered_again)
            .next()
            .is_some()
}

fn kuva_purchase_choices(
    vendors: &[EstimatedVendorPurchases],
    unexplained: i64,
) -> Vec<Vec<VendorKuvaBought>> {
    let Some((vendor, rest)) = vendors.split_first() else {
        return if unexplained == 0 {
            vec![Vec::new()]
        } else {
            Vec::new()
        };
    };

    let mut choices = kuva_purchase_choices(rest, unexplained);
    for &bought in &vendor.bought_counts {
        let kuva = i64::from(vendor.amount) * i64::from(bought);
        for mut choice in kuva_purchase_choices(rest, unexplained - kuva) {
            choice.push(VendorKuvaBought {
                manifest: vendor.manifest,
                bought,
            });
            choices.push(choice);
        }
    }

    choices
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::catalog::fixtures::{self, at};
    use crate::reset_checklist::context::fixtures::synced_at;

    const PALLADINO: &str = "Hubs/IronwakeDondaVendorManifest";

    fn bought(manifest: &'static str, bought: u32) -> VendorKuvaBought {
        VendorKuvaBought { manifest, bought }
    }

    fn weekly_purchase(item_id: &str, bought: u32) -> Value {
        json!({
            "Expiry": {"$date": {"$numberLong": "1789344000000"}},
            "NumPurchased": bought,
            "ItemId": item_id
        })
    }

    fn teshin_offer(bought: u32) -> Value {
        weekly_purchase("6a9a2f60000000000000f201", bought)
    }

    fn bought_since_fixture(
        teshin: &[Value],
        others: &[(&str, u32)],
        kuva: i64,
    ) -> Vec<VendorKuvaBought> {
        let next = inventory_with_purchases(teshin, others, kuva);

        kuva_bought_between(&fixtures::inventory(), &next, synced_at())
    }

    fn inventory_with_purchases(teshin: &[Value], others: &[(&str, u32)], kuva: i64) -> Inventory {
        fixtures::inventory_with(|value| {
            let vendors = fixtures::entries(value, "RecentVendorPurchases");
            vendors[0]["PurchaseHistory"] = json!(teshin);
            for (manifest, bought) in others {
                vendors.push(json!({
                    "VendorType": format!("/Lotus/Types/Game/VendorManifests/{manifest}"),
                    "PurchaseHistory": [weekly_purchase(manifest, *bought)]
                }));
            }
            fixtures::push_counted(value, "MiscItems", KUVA_ITEM, kuva);
        })
    }

    fn expiring(mut purchase: Value, millis: &str) -> Value {
        purchase["Expiry"]["$date"]["$numberLong"] = json!(millis);
        purchase
    }

    #[test]
    fn gain_left_by_confirmed_vendors() {
        let purchases = bought_since_fixture(
            &[teshin_offer(5)],
            &[("Hubs/MelicaVendorManifest", 2)],
            51_000,
        );

        assert_eq!(purchases, [bought(TESHIN_MANIFEST, 3)]);
    }

    #[test]
    fn two_estimated_vendors() {
        let purchases = bought_since_fixture(&[teshin_offer(5)], &[(PALLADINO, 1)], 65_000);

        assert_eq!(
            purchases,
            [bought(PALLADINO, 1), bought(TESHIN_MANIFEST, 3)]
        );
    }

    #[test]
    fn offers_bought_equally_often() {
        let teshin = [teshin_offer(3), weekly_purchase("other", 1)];

        assert_eq!(
            bought_since_fixture(&teshin, &[], 10_000),
            [bought(TESHIN_MANIFEST, 1)]
        );
    }

    #[test]
    fn count_over_the_kuva_limit() {
        let purchases = bought_since_fixture(&[teshin_offer(2)], &[(PALLADINO, 2)], 70_000);

        assert_eq!(purchases, []);
    }

    #[test]
    fn purchase_expiring_another_week() {
        let last_week = expiring(weekly_purchase("other", 1), "1788739200000");
        let teshin = [teshin_offer(2), last_week];

        assert_eq!(bought_since_fixture(&teshin, &[], 10_000), []);
    }

    #[test]
    fn gain_with_other_kuva() {
        assert_eq!(bought_since_fixture(&[teshin_offer(5)], &[], 30_650), []);
    }

    #[test]
    fn teshin_rotating_kuva() {
        let kuva_week = at(1_790_000_000_000);
        let weekly = expiring(teshin_offer(25), "1790553600000");
        let rotating = |expiry| expiring(weekly_purchase("rotating", 1), expiry);
        let nothing_bought = fixtures::inventory();
        let rotating_bought = inventory_with_purchases(&[rotating("1794787200000")], &[], 50_000);
        let weekly_bought = inventory_with_purchases(std::slice::from_ref(&weekly), &[], 250_000);
        let both_bought =
            inventory_with_purchases(&[weekly, rotating("1794787200000")], &[], 300_000);
        let another_week = [teshin_offer(5), rotating("1793577600000")];

        for previous in [&nothing_bought, &rotating_bought] {
            assert_eq!(
                kuva_bought_between(previous, &both_bought, kuva_week),
                [bought(TESHIN_MANIFEST, 25)]
            );
        }
        assert_eq!(
            kuva_bought_between(&nothing_bought, &weekly_bought, kuva_week),
            [bought(TESHIN_MANIFEST, 25)]
        );
        assert_eq!(bought_since_fixture(&another_week, &[], 80_000), []);
    }
}
