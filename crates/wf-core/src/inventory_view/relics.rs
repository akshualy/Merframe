use wf_data::Refinement;

use crate::catalog::{
    VaultStatus, display_name_from_path, refinement_from_unique_name, refinement_name,
};
use crate::view::View;

use super::RelicRow;

const PROJECTION_TIERS: [(&str, &str); 6] = [
    ("T0", "Void"),
    ("T1", "Lith"),
    ("T2", "Meso"),
    ("T3", "Neo"),
    ("T4", "Axi"),
    ("T5", "Requiem"),
];

fn projection_identity(unique_name: &str) -> (String, String) {
    let leaf = unique_name
        .rsplit_once('/')
        .map_or(unique_name, |(_, leaf)| leaf);
    let Some((tier, rest)) = PROJECTION_TIERS
        .into_iter()
        .find_map(|(code, tier)| leaf.strip_prefix(code).map(|rest| (tier, rest)))
    else {
        return (display_name_from_path(leaf), String::new());
    };
    let rest = rest.strip_prefix("VoidProjection").unwrap_or(rest);
    let designation = ["Bronze", "Silver", "Gold", "Platinum"]
        .into_iter()
        .find_map(|suffix| rest.strip_suffix(suffix))
        .unwrap_or(rest);
    if designation.is_empty() {
        return (tier.to_owned(), tier.to_owned());
    }
    (
        format!("{tier} {}", display_name_from_path(designation)),
        tier.to_owned(),
    )
}

pub(crate) fn relics(view: &View) -> Vec<RelicRow> {
    let View {
        inventory,
        catalog,
        prices,
        favourites,
        listings,
    } = *view;
    let mut rows: Vec<(Refinement, RelicRow)> = inventory
        .relics()
        .filter(|(_, count)| *count > 0)
        .filter_map(|(unique_name, count)| {
            let known = catalog.relic_by_unique_name(unique_name);
            if known.is_some_and(|(relic, _)| !relic.tradable) {
                return None;
            }
            let (relic, tier, refinement) = if let Some((relic, refinement)) = known {
                (relic.name.clone(), relic.tier.clone(), refinement)
            } else {
                let (relic, tier) = projection_identity(unique_name);
                let refinement =
                    refinement_from_unique_name(unique_name).unwrap_or(Refinement::Intact);
                (relic, tier, refinement)
            };
            let market_name = known
                .and_then(|(relic, _)| relic.market_info.as_ref())
                .map(|info| info.url_name.as_str());
            Some((
                refinement,
                RelicRow {
                    relic,
                    tier,
                    refinement: refinement_name(refinement),
                    image_name: known.and_then(|(relic, refinement)| {
                        relic.image_names.get(&refinement).cloned()
                    }),
                    count,
                    vault: VaultStatus::from(known.map(|(relic, _)| relic.vaulted)),
                    plat: market_name.and_then(|url_name| prices.plat(url_name)),
                    favourite: favourites.contains(unique_name),
                    order_placed: market_name.is_some_and(|url_name| listings.has_order(url_name)),
                    unique_name: unique_name.to_owned(),
                },
            ))
        })
        .collect();
    rows.sort_by(|(left_refinement, left), (right_refinement, right)| {
        left.relic
            .cmp(&right.relic)
            .then(left_refinement.cmp(right_refinement))
    });
    rows.into_iter().map(|(_, row)| row).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::tests::{no_listings, prices};
    use super::*;
    use crate::catalog::{Catalog, fixtures};
    use crate::favourites::Favourites;

    const RELIC_PROJECTIONS: &str = include_str!("../../../../fixtures/relic_projections.json");

    fn projection_catalog() -> Catalog {
        Catalog::from_json(fixtures::ITEMS, RELIC_PROJECTIONS).unwrap()
    }

    #[test]
    fn every_owned_relic_resolves() {
        let inventory = fixtures::inventory();
        let catalog = projection_catalog();
        let rows = relics(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let derived: Vec<&str> = rows
            .iter()
            .filter(|row| catalog.relic_by_unique_name(&row.unique_name).is_none())
            .map(|row| row.unique_name.as_str())
            .collect();
        assert!(derived.is_empty(), "{derived:?}");
        assert_eq!(rows.len(), 25);
        assert!(rows.iter().any(|row| row.vault == VaultStatus::Vaulted));
        assert!(rows.iter().any(|row| row.vault == VaultStatus::Available));
    }

    #[test]
    fn lith_g12_from_sevagoth_projection() {
        let inventory = fixtures::inventory();
        let catalog = projection_catalog();
        let row = relics(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        })
        .into_iter()
        .find(|row| {
            row.unique_name == "/Lotus/Types/Game/Projections/T1VoidProjectionSevagothPrimeDBronze"
        })
        .unwrap();
        assert_eq!(row.relic, "Lith G12");
        assert_eq!(row.tier, "Lith");
        assert_eq!(row.refinement, "Intact");
        assert_eq!(row.count, 25);
    }

    #[test]
    fn one_row_per_refinement() {
        let inventory = fixtures::inventory();
        let catalog = projection_catalog();
        let rows = relics(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let counted = |refinement: &str| {
            rows.iter()
                .filter(|row| row.refinement == refinement)
                .count()
        };
        assert_eq!(counted("Intact"), 16);
        assert_eq!(counted("Exceptional"), 3);
        assert_eq!(counted("Flawless"), 2);
        assert_eq!(counted("Radiant"), 4);

        let refined: Vec<&RelicRow> = rows.iter().filter(|row| row.relic == "Lith S18").collect();
        let refinements: Vec<&str> = refined.iter().map(|row| row.refinement).collect();
        assert_eq!(
            refinements,
            ["Intact", "Exceptional", "Flawless", "Radiant"]
        );
        let names: HashSet<&str> = refined.iter().map(|row| row.unique_name.as_str()).collect();
        assert_eq!(names.len(), refined.len());
    }

    #[test]
    fn untradable_relics_dropped() {
        let inventory = fixtures::inventory();
        let catalog = projection_catalog();
        let rows = relics(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert!(
            !rows
                .iter()
                .any(|row| row.unique_name.ends_with("T5VoidProjectionImmortalOmniA")),
            "the eterna relic cannot be traded"
        );
        let requiem = rows
            .iter()
            .find(|row| row.relic == "Requiem I")
            .expect("Requiem I");
        assert_eq!(requiem.tier, "Requiem");
        assert_eq!(requiem.count, 8);
    }

    #[test]
    fn projection_identity_fallback() {
        assert_eq!(
            projection_identity(
                "/Lotus/Types/Game/Projections/T1VoidProjectionSevagothPrimeDBronze"
            ),
            (String::from("Lith Sevagoth Prime D"), String::from("Lith"))
        );
        assert_eq!(
            projection_identity("/Lotus/Types/Game/Projections/T4VoidProjectionPPlatinum"),
            (String::from("Axi P"), String::from("Axi"))
        );
        assert_eq!(
            projection_identity("/Lotus/Types/Game/Projections/T5VoidProjectionImmortalOmniA"),
            (
                String::from("Requiem Immortal Omni A"),
                String::from("Requiem")
            )
        );
    }

    #[test]
    fn axi_a21_row() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let rows = relics(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert!(rows.iter().all(|row| row.count > 0));

        let known = rows
            .iter()
            .find(|row| row.vault != VaultStatus::Unknown)
            .expect("known relic");
        assert_eq!(known.relic, "Axi A21");
        assert_eq!(known.tier, "Axi");
        assert_eq!(known.refinement, "Intact");
        assert_eq!(known.count, 3);
        assert_eq!(known.vault, VaultStatus::Available);
        assert_eq!(known.plat, Some(5.0));
        assert!(known.image_name.is_some());
    }
}
