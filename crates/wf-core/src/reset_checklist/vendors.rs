use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Serialize;
use wf_inventory::VendorPurchaseHistory;
use wf_worldstate::game_week_count;

use crate::reset_checklist::cadence::ResetCadence;
use crate::reset_checklist::context::ResetChecklistContext;
use crate::reset_checklist::trackers::tally;
use crate::resources::archon_shard_type;

const AMBER_SHARD_WEEK: i64 = 665;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ArchonShardColour {
    Azure,
    Amber,
    Crimson,
}

impl ArchonShardColour {
    fn of_week(now: DateTime<Utc>) -> Self {
        match (game_week_count(now) - AMBER_SHARD_WEEK).rem_euclid(3) {
            0 => Self::Amber,
            1 => Self::Crimson,
            _ => Self::Azure,
        }
    }

    fn item_type(self) -> String {
        archon_shard_type(match self {
            Self::Azure => "Boreal",
            Self::Amber => "Nira",
            Self::Crimson => "Amar",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VendorWeeklyOffer {
    Kuva {
        amount: u32,
        bought: u32,
        limit: u32,
    },
    ArchonShard {
        colour: ArchonShardColour,
        image_name: Option<String>,
        bought: u32,
        limit: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ResetChecklistVendor {
    pub name: &'static str,
    pub place: Option<&'static str>,
    pub offers_bought: u32,
    pub weekly_offer: Option<VendorWeeklyOffer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct KuvaWeeklySummary {
    pub bought: u32,
    pub available: u32,
}

#[derive(Clone, Copy)]
enum NamedOffer {
    Kuva { amount: u32, limit: u32 },
    ArchonShard { limit: u32 },
}

struct VendorDefinition {
    manifest: &'static str,
    name: &'static str,
    place: Option<&'static str>,
    offer: Option<NamedOffer>,
}

const VENDORS: [VendorDefinition; 9] = [
    VendorDefinition {
        manifest: "Hubs/TeshinHardModeVendorManifest",
        name: "Teshin",
        place: Some("The Steel Path Honors"),
        offer: None,
    },
    VendorDefinition {
        manifest: "Hubs/IronwakeDondaVendorManifest",
        name: "Palladino",
        place: Some("Iron Wake"),
        offer: None,
    },
    VendorDefinition {
        manifest: "Zariman/ArchimedeanVendorManifest",
        name: "Archimedean Yonta",
        place: Some("Chrysalith"),
        offer: Some(NamedOffer::Kuva {
            amount: 35_000,
            limit: 1,
        }),
    },
    VendorDefinition {
        manifest: "EntratiLabs/EntratiLabVendorManifest",
        name: "Bird 3",
        place: Some("Sanctum Anatomica"),
        offer: Some(NamedOffer::ArchonShard { limit: 1 }),
    },
    VendorDefinition {
        manifest: "Duviri/AcrithisVendorManifest",
        name: "Acrithis",
        place: Some("Dormizone"),
        offer: None,
    },
    VendorDefinition {
        manifest: "Solaris/NightcapVendorManifest",
        name: "Nightcap",
        place: Some("Fortuna"),
        offer: None,
    },
    VendorDefinition {
        manifest: "TheHex/Temple1999VendorManifest",
        name: "Flare",
        place: Some("Hollvania"),
        offer: None,
    },
    VendorDefinition {
        manifest: "Hubs/AspirantZorbaVendorManifest",
        name: "Aspirant Zorba",
        place: Some("Relays"),
        offer: Some(NamedOffer::Kuva {
            amount: 6_000,
            limit: 7,
        }),
    },
    VendorDefinition {
        manifest: "Hubs/MelicaVendorManifest",
        name: "Cephalon Melica",
        place: None,
        offer: Some(NamedOffer::Kuva {
            amount: 10_500,
            limit: 4,
        }),
    },
];

pub(crate) fn vendors(context: &ResetChecklistContext) -> Vec<ResetChecklistVendor> {
    VENDORS
        .iter()
        .map(|vendor| {
            let history = context
                .inventory
                .recent_vendor_purchases
                .iter()
                .find(|history| history.vendor_type.ends_with(vendor.manifest));
            vendor_this_week(vendor, history, context)
        })
        .collect()
}

fn vendor_this_week(
    vendor: &VendorDefinition,
    history: Option<&VendorPurchaseHistory>,
    context: &ResetChecklistContext,
) -> ResetChecklistVendor {
    let reset = ResetCadence::Weekly.window(context.now).expiry;
    let weekly: Vec<_> = history
        .into_iter()
        .flat_map(|history| history.active_purchases(context.now))
        .filter(|purchase| purchase.expiry.datetime() == reset)
        .collect();
    let offers: HashSet<&str> = weekly
        .iter()
        .map(|purchase| purchase.item_id.as_str())
        .collect();
    let bought = weekly.iter().map(|purchase| purchase.num_purchased).sum();

    ResetChecklistVendor {
        name: vendor.name,
        place: vendor.place,
        offers_bought: tally(offers),
        weekly_offer: vendor.offer.map(|offer| match offer {
            NamedOffer::Kuva { amount, limit } => VendorWeeklyOffer::Kuva {
                amount,
                bought,
                limit,
            },
            NamedOffer::ArchonShard { limit } => {
                let colour = ArchonShardColour::of_week(context.now);
                VendorWeeklyOffer::ArchonShard {
                    colour,
                    image_name: context
                        .items
                        .lookup(&colour.item_type())
                        .and_then(|shard| shard.image_name.clone()),
                    bought,
                    limit,
                }
            }
        }),
    }
}

pub(crate) fn kuva_summary(vendors: &[ResetChecklistVendor]) -> KuvaWeeklySummary {
    vendors
        .iter()
        .filter_map(|vendor| match vendor.weekly_offer {
            Some(VendorWeeklyOffer::Kuva {
                amount,
                bought,
                limit,
            }) => Some((amount, bought.min(limit), limit)),
            Some(VendorWeeklyOffer::ArchonShard { .. }) | None => None,
        })
        .fold(
            KuvaWeeklySummary {
                bought: 0,
                available: 0,
            },
            |summary, (amount, bought, limit)| KuvaWeeklySummary {
                bought: summary.bought + amount * bought,
                available: summary.available + amount * limit,
            },
        )
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use serde_json::json;

    use super::*;
    use crate::catalog::fixtures::{self, at};
    use crate::reset_checklist::context::fixtures::{context, synced_at};

    #[test]
    fn weekly_purchases_only() {
        let inventory = fixtures::inventory();
        let vendors = vendors(&context(&inventory, synced_at()));

        let named = |name| vendors.iter().find(|vendor| vendor.name == name).unwrap();

        assert_eq!(vendors.len(), VENDORS.len());
        assert_eq!(named("Teshin").offers_bought, 1);
        assert_eq!(named("Palladino").offers_bought, 0);
        assert_eq!(
            named("Bird 3").weekly_offer,
            Some(VendorWeeklyOffer::ArchonShard {
                colour: ArchonShardColour::Amber,
                image_name: Some("ArchonShardNira.png".to_owned()),
                bought: 0,
                limit: 1,
            })
        );
        assert_eq!(
            named("Archimedean Yonta").weekly_offer,
            Some(VendorWeeklyOffer::Kuva {
                amount: 35_000,
                bought: 1,
                limit: 1,
            })
        );
        assert_eq!(
            kuva_summary(&vendors),
            KuvaWeeklySummary {
                bought: 35_000,
                available: 119_000,
            }
        );
    }

    #[test]
    fn kuva_bought_capped_at_limit() {
        let inventory = fixtures::inventory_with(|value| {
            fixtures::entries(value, "RecentVendorPurchases").push(json!({
                "VendorType": "/Lotus/Types/Game/VendorManifests/Hubs/MelicaVendorManifest",
                "PurchaseHistory": [{
                    "Expiry": {"$date": {"$numberLong": "1789344000000"}},
                    "NumPurchased": 5,
                    "ItemId": "6a9a2f60000000000000f205"
                }]
            }));
        });

        let vendors = vendors(&context(&inventory, synced_at()));

        assert_eq!(kuva_summary(&vendors).bought, 35_000 + 4 * 10_500);
    }

    #[test]
    fn archon_shard_rotation() {
        let monday = at(1_794_182_400_000);

        let colours: Vec<_> = (0..3)
            .map(|week| ArchonShardColour::of_week(monday + TimeDelta::weeks(week)))
            .collect();

        assert_eq!(
            colours,
            [
                ArchonShardColour::Amber,
                ArchonShardColour::Crimson,
                ArchonShardColour::Azure
            ]
        );
    }
}
