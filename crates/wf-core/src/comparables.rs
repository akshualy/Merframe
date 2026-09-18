use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use wf_market::{Polarity, RivenAttribute, RivenAuction, WeaponAuctions};

const ABBREVIATIONS: [(&str, &str); 32] = [
    ("critical_chance", "CC"),
    ("critical_damage", "CD"),
    ("base_damage_/_melee_damage", "DMG"),
    ("multishot", "MS"),
    ("fire_rate_/_attack_speed", "FR"),
    ("reload_speed", "RLS"),
    ("toxin_damage", "TOX"),
    ("heat_damage", "HEAT"),
    ("cold_damage", "COLD"),
    ("electric_damage", "ELEC"),
    ("damage_vs_corpus", "DTC"),
    ("damage_vs_infested", "DTI"),
    ("damage_vs_grineer", "DTG"),
    ("puncture_damage", "PUNC"),
    ("impact_damage", "IMP"),
    ("slash_damage", "SLASH"),
    ("magazine_capacity", "MAG"),
    ("ammo_maximum", "AMMO"),
    ("recoil", "REC"),
    ("zoom", "ZOOM"),
    ("status_chance", "SC"),
    ("status_duration", "SD"),
    ("punch_through", "PT"),
    ("projectile_speed", "PFS"),
    ("range", "RANGE"),
    ("channeling_damage", "IC"),
    ("channeling_efficiency", "EFF"),
    ("critical_chance_on_slide_attack", "SLIDE"),
    ("finisher_damage", "FIN"),
    ("combo_duration", "CDUR"),
    ("chance_to_gain_extra_combo_count", "XCC"),
    ("chance_to_gain_combo_count", "GCC"),
];

fn abbreviation(slug: &str) -> Option<&'static str> {
    ABBREVIATIONS
        .iter()
        .find(|(known, _)| *known == slug)
        .map(|(_, short)| *short)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ComparedStat {
    pub slug: String,
    pub positive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparableAttribute {
    pub name: String,
    pub abbr: Option<String>,
    pub unit: Option<String>,
    pub value: f64,
    pub positive: bool,
    pub shared: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparableListing {
    pub id: String,
    pub price: u32,
    pub direct_sell: bool,
    pub name: String,
    pub rerolls: u32,
    pub rank: u32,
    pub mastery: u32,
    pub polarity: Polarity,
    pub similarity: f64,
    pub attributes: Vec<ComparableAttribute>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RivenComparables {
    pub weapon_slug: String,
    pub updated_at: i64,
    pub truncated: bool,
    pub listed: usize,
    pub lowest: Option<u32>,
    pub listings: Vec<ComparableListing>,
}

type SignedStat = (String, bool);

fn auction_stats(auction: &RivenAuction) -> BTreeSet<SignedStat> {
    auction
        .attributes
        .iter()
        .map(|attribute| (attribute.url_name.clone(), attribute.positive))
        .collect()
}

#[allow(clippy::cast_precision_loss, reason = "a riven has at most four stats")]
fn similarity(shown: &BTreeSet<SignedStat>, offered: &BTreeSet<SignedStat>) -> f64 {
    let union = shown.union(offered).count();
    if union == 0 {
        return 0.0;
    }
    shown.intersection(offered).count() as f64 / union as f64
}

fn price(auction: &RivenAuction) -> u32 {
    match auction.buyout_price {
        Some(buyout) if auction.is_direct_sell => buyout,
        _ => auction.starting_price,
    }
}

fn listing(
    auction: &RivenAuction,
    shown: &BTreeSet<SignedStat>,
    names: &HashMap<&str, &RivenAttribute>,
) -> ComparableListing {
    let offered = auction_stats(auction);
    ComparableListing {
        id: auction.id.clone(),
        price: price(auction),
        direct_sell: auction.is_direct_sell,
        name: auction.name.clone(),
        rerolls: auction.re_rolls,
        rank: auction.mod_rank,
        mastery: auction.mastery_level,
        polarity: auction.polarity,
        similarity: similarity(shown, &offered),
        attributes: auction
            .attributes
            .iter()
            .map(|attribute| {
                let known = names.get(attribute.url_name.as_str());
                ComparableAttribute {
                    name: known
                        .and_then(|known| known.i18n.get("en"))
                        .map_or_else(|| attribute.url_name.clone(), |text| text.name.clone()),
                    abbr: abbreviation(&attribute.url_name).map(str::to_owned),
                    unit: known.and_then(|known| known.unit.clone()),
                    value: attribute.value,
                    positive: attribute.positive,
                    shared: shown.contains(&(attribute.url_name.clone(), attribute.positive)),
                }
            })
            .collect(),
    }
}

pub(crate) fn comparables(
    stats: &[ComparedStat],
    auctions: &WeaponAuctions,
    attributes: &[RivenAttribute],
) -> RivenComparables {
    let names: HashMap<&str, &RivenAttribute> = attributes
        .iter()
        .map(|attribute| (attribute.slug.as_str(), attribute))
        .collect();
    let shown: BTreeSet<SignedStat> = stats
        .iter()
        .map(|stat| (stat.slug.clone(), stat.positive))
        .collect();
    let mut listings: Vec<ComparableListing> = auctions
        .auctions
        .iter()
        .map(|auction| listing(auction, &shown, &names))
        .filter(|listing| listing.similarity >= 0.5)
        .collect();
    listings.sort_by(|left, right| {
        right
            .similarity
            .total_cmp(&left.similarity)
            .then(left.price.cmp(&right.price))
    });
    listings.truncate(25);
    RivenComparables {
        weapon_slug: auctions.slug.clone(),
        updated_at: auctions.updated_at,
        truncated: auctions.truncated,
        listed: auctions.auctions.len(),
        lowest: auctions.auctions.iter().map(price).min(),
        listings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wf_market::{RivenAttribute, envelope, parse_riven_auctions};

    const AUCTIONS: &str = include_str!("../../../fixtures/riven_auctions.json");
    const ATTRIBUTES: &str = include_str!("../../wf-market/tests/fixtures/riven_attributes.json");

    fn compared(stats: &[(&str, bool)]) -> RivenComparables {
        let auctions = parse_riven_auctions(AUCTIONS).unwrap();
        let attributes = envelope::<Vec<RivenAttribute>>(ATTRIBUTES).unwrap();
        let shown: Vec<ComparedStat> = stats
            .iter()
            .map(|(slug, positive)| ComparedStat {
                slug: (*slug).to_owned(),
                positive: *positive,
            })
            .collect();
        comparables(&shown, &auctions, &attributes)
    }

    #[test]
    fn exact_match_leads() {
        let found = compared(&[
            ("damage_vs_grineer", true),
            ("critical_chance", true),
            ("damage_vs_corpus", true),
        ]);

        assert_eq!(found.weapon_slug, "torid");
        assert_eq!(found.listed, 40);
        assert_eq!(found.lowest, Some(330));
        let first = &found.listings[0];
        assert_eq!(first.id, "1936123ea9999d41be532edf");
        assert!((first.similarity - 1.0).abs() < f64::EPSILON);
        assert_eq!(first.price, 330);
        assert!(first.attributes.iter().all(|attribute| attribute.shared));
        assert_eq!(first.attributes[1].name, "Critical Chance");
        assert_eq!(first.attributes[1].unit.as_deref(), Some("percent"));
        assert_eq!(first.attributes[1].abbr.as_deref(), Some("CC"));
        assert_eq!(first.attributes[0].abbr.as_deref(), Some("DTG"));
        assert_eq!(first.attributes[0].unit.as_deref(), Some("multiply"));
        assert!(
            found
                .listings
                .windows(2)
                .all(|pair| pair[0].similarity >= pair[1].similarity)
        );
    }

    #[test]
    fn negative_is_its_own_stat() {
        let found = compared(&[
            ("critical_chance", true),
            ("damage_vs_corpus", true),
            ("damage_vs_grineer", false),
        ]);

        let first = &found.listings[0];
        assert!(first.similarity < 1.0);
        assert!(
            found
                .listings
                .iter()
                .all(|listing| listing.similarity >= 0.5)
        );
        assert!(found.listings.len() <= 25);
    }

    #[test]
    fn unrelated_roll() {
        let found = compared(&[("range", true), ("combo_efficiency", true)]);

        assert!(found.listings.is_empty());
        assert_eq!(found.lowest, Some(330));
    }
}
