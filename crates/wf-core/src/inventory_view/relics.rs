use wf_data::{Refinement, projection_family};

use crate::catalog::{VaultStatus, display_name_from_path};
use crate::identity::ItemKind;
use crate::view::View;

use super::{ItemIndex, ItemSummary, RelicRow};

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
    let designation = projection_family(rest);
    if designation.is_empty() {
        return (tier.to_owned(), tier.to_owned());
    }
    (
        format!("{tier} {}", display_name_from_path(designation)),
        tier.to_owned(),
    )
}

pub(crate) fn relics(view: &View, index: &mut ItemIndex) -> Vec<RelicRow> {
    let View {
        account,
        catalog,
        items,
        prices,
        favourites,
        ..
    } = *view;
    let inventory = &account.inventory;
    let mut rows: Vec<RelicRow> = inventory
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
                    Refinement::of_projection(unique_name).unwrap_or(Refinement::Intact);
                (relic, tier, refinement)
            };

            let summary = match items
                .get(unique_name)
                .filter(|record| matches!(record.kind, ItemKind::Relic { .. }))
            {
                Some(record) => ItemSummary {
                    name: relic,
                    ..ItemSummary::from(record)
                },
                None => ItemSummary {
                    unique_name: unique_name.to_owned(),
                    name: relic,
                    image_name: None,
                    market_slug: None,
                    prime: false,
                    vault: Some(VaultStatus::Unknown),
                    ducats: None,
                },
            };
            let plat = summary
                .market_slug
                .as_deref()
                .and_then(|slug| prices.plat(slug));
            Some(RelicRow {
                item: index.add_marked(summary, favourites.contains(unique_name)),
                tier,
                refinement,
                count,
                plat,
            })
        })
        .collect();
    rows.sort_by(|left, right| {
        index[left.item]
            .name
            .cmp(&index[right.item].name)
            .then(left.refinement.cmp(&right.refinement))
    });
    rows
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::tests::priced;
    use super::*;
    use crate::catalog::{Catalog, fixtures};
    use crate::view::Fixture;

    const RELIC_PROJECTIONS: &str = include_str!("../../../../fixtures/relic_projections.json");

    fn projections() -> Fixture {
        let catalog =
            Catalog::from_json(fixtures::ITEMS, RELIC_PROJECTIONS, fixtures::COMPONENTS).unwrap();
        priced(catalog, fixtures::inventory())
    }

    #[test]
    fn every_owned_relic_resolves() {
        let fixture = projections();
        let (rows, index) = fixture.rows(relics);
        let derived: Vec<&str> = rows
            .iter()
            .filter(|row| {
                fixture
                    .catalog
                    .relic_by_unique_name(&index[row.item].unique_name)
                    .is_none()
            })
            .map(|row| index[row.item].unique_name.as_str())
            .collect();
        assert!(derived.is_empty(), "{derived:?}");
        assert_eq!(rows.len(), 25);
        assert!(
            rows.iter()
                .any(|row| index[row.item].vault == Some(VaultStatus::Vaulted))
        );
        assert!(
            rows.iter()
                .any(|row| index[row.item].vault == Some(VaultStatus::Available))
        );
    }

    #[test]
    fn lith_g12_from_sevagoth_projection() {
        let fixture = projections();
        let (rows, index) = fixture.rows(relics);
        let row = rows
            .into_iter()
            .find(|row| {
                index[row.item].unique_name
                    == "/Lotus/Types/Game/Projections/T1VoidProjectionSevagothPrimeDBronze"
            })
            .unwrap();
        assert_eq!(index[row.item].name, "Lith G12");
        assert_eq!(row.tier, "Lith");
        assert_eq!(row.refinement, Refinement::Intact);
        assert_eq!(row.count, 25);
    }

    #[test]
    fn one_row_per_refinement() {
        let fixture = projections();
        let (rows, index) = fixture.rows(relics);
        let counted = |refinement: Refinement| {
            rows.iter()
                .filter(|row| row.refinement == refinement)
                .count()
        };
        assert_eq!(counted(Refinement::Intact), 16);
        assert_eq!(counted(Refinement::Exceptional), 3);
        assert_eq!(counted(Refinement::Flawless), 2);
        assert_eq!(counted(Refinement::Radiant), 4);

        let refined: Vec<&RelicRow> = rows
            .iter()
            .filter(|row| index[row.item].name == "Lith S18")
            .collect();
        let refinements: Vec<Refinement> = refined.iter().map(|row| row.refinement).collect();
        assert_eq!(refinements, Refinement::ALL);
        let names: HashSet<&str> = refined
            .iter()
            .map(|row| index[row.item].unique_name.as_str())
            .collect();
        assert_eq!(names.len(), refined.len());
    }

    #[test]
    fn untradable_relics_dropped() {
        let fixture = projections();
        let (rows, index) = fixture.rows(relics);
        assert!(
            !rows.iter().any(|row| index[row.item]
                .unique_name
                .ends_with("T5VoidProjectionImmortalOmniA")),
            "the eterna relic cannot be traded"
        );
        let requiem = rows
            .iter()
            .find(|row| index[row.item].name == "Requiem I")
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
        let fixture = priced(fixtures::catalog(), fixtures::inventory());
        let (rows, index) = fixture.rows(relics);
        assert!(rows.iter().all(|row| row.count > 0));

        let known = rows
            .iter()
            .find(|row| index[row.item].vault != Some(VaultStatus::Unknown))
            .expect("known relic");
        assert_eq!(index[known.item].name, "Axi A21");
        assert_eq!(known.tier, "Axi");
        assert_eq!(known.refinement, Refinement::Intact);
        assert_eq!(known.count, 3);
        assert_eq!(index[known.item].vault, Some(VaultStatus::Available));
        assert_eq!(known.plat, Some(5.0));
        assert_eq!(
            index[known.item].market_slug.as_deref(),
            Some("axi_a21_relic")
        );
        assert!(index[known.item].image_name.is_some());
        let unknown = rows
            .iter()
            .find(|row| index[row.item].vault == Some(VaultStatus::Unknown))
            .expect("relic outside the export");
        assert_eq!(index[unknown.item].market_slug, None);
    }
}
