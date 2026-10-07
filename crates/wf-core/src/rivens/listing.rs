use serde::Deserialize;
use wf_data::rank_multiplier;
use wf_market::{
    CreateAuctionItem, CreateAuctionRequest, RivenAttributeInstance, UpdateAuctionRequest,
};

use super::RivenRow;
use super::grading::{AttributeGrade, display_value};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ListingChoices {
    pub direct: bool,
    pub selling_price: u32,
    pub starting_price: u32,
    pub buyout_price: u32,
    pub min_reputation: u32,
    pub note: String,
    pub visible: bool,
    pub rank: u32,
}

impl ListingChoices {
    fn starting(&self) -> u32 {
        if self.direct {
            self.selling_price
        } else {
            self.starting_price
        }
    }

    fn buyout(&self) -> Option<u32> {
        let asking = if self.direct {
            self.selling_price
        } else {
            self.buyout_price
        };
        (asking > 0).then_some(asking)
    }

    fn reputation(&self) -> u32 {
        if self.direct { 0 } else { self.min_reputation }
    }
}

pub fn listing_update(choices: &ListingChoices) -> UpdateAuctionRequest {
    UpdateAuctionRequest {
        buyout_price: Some(choices.buyout()),
        starting_price: Some(choices.starting()),
        minimal_reputation: Some(choices.reputation()),
        note: Some(choices.note.clone()),
        visible: Some(choices.visible),
    }
}

pub(super) fn unlisted_stat(attributes: &[AttributeGrade]) -> Option<String> {
    let unknown = attributes
        .iter()
        .find(|attribute| attribute.slug.is_none())?;
    Some(unknown.name.clone().unwrap_or_else(|| unknown.tag.clone()))
}

pub fn listing_payload(row: &RivenRow, choices: &ListingChoices) -> Option<CreateAuctionRequest> {
    let mut attributes = Vec::with_capacity(row.attributes.len());
    for attribute in &row.attributes {
        let Some(url_name) = attribute.slug.clone() else {
            continue;
        };
        let value = display_value(
            attribute.rolled? * rank_multiplier(choices.rank),
            attribute.unit.as_deref() == Some("multiply"),
        );
        attributes.push(RivenAttributeInstance {
            value,
            positive: !attribute.curse,
            url_name,
        });
    }
    if attributes.is_empty() {
        return None;
    }
    Some(CreateAuctionRequest {
        item: CreateAuctionItem {
            attributes,
            polarity: row.polarity?,
            mod_rank: choices.rank,
            name: row.name.clone()?,
            re_rolls: row.rerolls,
            mastery_level: row.rank_required?,
            weapon_url_name: row.weapon_slug.clone()?,
        },
        buyout_price: choices.buyout(),
        starting_price: choices.starting(),
        minimal_reputation: Some(choices.reputation()),
        note: choices.note.clone(),
        visible: choices.visible,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rivens::AttributeGrade;
    use crate::rivens::grading::round_to;
    use crate::rivens::support::*;
    use wf_inventory::RivenFingerprint;
    use wf_market::Polarity;

    fn direct(price: u32) -> ListingChoices {
        ListingChoices {
            direct: true,
            selling_price: price,
            ..ListingChoices::default()
        }
    }

    #[test]
    fn unresolved_row_has_no_payload() {
        let bare = RivenRow {
            item_id: "0".to_owned(),
            item_type: RIFLE_RIVEN.to_owned(),
            riven_type: None,
            weapon_class: None,
            name: None,
            weapon: None,
            weapon_path: None,
            listed_in_wfm: false,
            weapon_slug: None,
            image_name: None,
            disposition: None,
            disposition_weapon: None,
            unveiled: true,
            rank: 0,
            rank_required: None,
            rerolls: 0,
            polarity: None,
            grade: 0.0,
            attributes: Vec::new(),
            good_roll: None,
            unlisted_stat: None,
            pending: None,
        };
        assert!(listing_payload(&bare, &direct(100)).is_none());
    }

    #[test]
    fn auction_payload() {
        let tab = riven_tab();
        let cernos = by_weapon(&tab.unveiled, "Proboscis Cernos");
        let auction = ListingChoices {
            direct: false,
            starting_price: 200,
            buyout_price: 400,
            min_reputation: 7,
            note: "merframe".to_owned(),
            visible: true,
            rank: cernos.rank,
            ..ListingChoices::default()
        };
        let payload = listing_payload(cernos, &auction).unwrap();
        assert_eq!(payload.starting_price, 200);
        assert_eq!(payload.buyout_price, Some(400));
        assert_eq!(payload.minimal_reputation, Some(7));
        assert_eq!(payload.note, "merframe");
        assert!(payload.visible);
        assert_eq!(payload.item.mod_rank, 8);
        assert_eq!(payload.item.re_rolls, cernos.rerolls);
        assert_eq!(payload.item.name.as_str(), "Sci-cronicron");
        assert_eq!(payload.item.weapon_url_name.as_str(), "proboscis_cernos");
        assert_eq!(payload.item.attributes.len(), 4);
        assert_eq!(
            payload
                .item
                .attributes
                .iter()
                .filter(|attribute| !attribute.positive)
                .count(),
            1
        );
        assert!(
            payload
                .item
                .attributes
                .iter()
                .any(|attribute| attribute.url_name == "slash_damage"
                    && (attribute.value - 108.7).abs() < f64::EPSILON)
        );
        assert!(
            payload
                .item
                .attributes
                .iter()
                .all(|attribute| attribute.positive == (attribute.value > 0.0))
        );
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["item"]["type"], "riven");
    }

    #[test]
    fn direct_sale_payload() {
        let tab = riven_tab();
        let cernos = by_weapon(&tab.unveiled, "Proboscis Cernos");
        let payload = listing_payload(cernos, &direct(120)).unwrap();
        assert_eq!(payload.starting_price, 120);
        assert_eq!(payload.buyout_price, Some(120));
        assert_eq!(payload.minimal_reputation, Some(0));
        assert!(!payload.visible);
        assert_eq!(serde_json::to_value(&payload).unwrap()["note"], "");

        let free = listing_payload(
            cernos,
            &ListingChoices {
                direct: false,
                starting_price: 90,
                ..ListingChoices::default()
            },
        )
        .unwrap();
        assert_eq!(free.starting_price, 90);
        assert_eq!(
            free.buyout_price, None,
            "an auction without a buyout leaves the field out"
        );
    }

    #[test]
    fn update_from_choices() {
        let auction = listing_update(&ListingChoices {
            direct: false,
            starting_price: 200,
            min_reputation: 4,
            visible: true,
            ..ListingChoices::default()
        });
        assert_eq!(auction.starting_price, Some(200));
        assert_eq!(auction.buyout_price, Some(None));
        assert_eq!(auction.minimal_reputation, Some(4));
        assert_eq!(auction.note.as_deref(), Some(""));
        assert_eq!(auction.visible, Some(true));

        let sale = listing_update(&ListingChoices {
            min_reputation: 4,
            note: "merframe".to_owned(),
            ..direct(150)
        });
        assert_eq!(sale.starting_price, Some(150));
        assert_eq!(sale.buyout_price, Some(Some(150)));
        assert_eq!(sale.minimal_reputation, Some(0));
        assert_eq!(sale.note.as_deref(), Some("merframe"));
        assert_eq!(sale.visible, Some(false));
    }

    #[test]
    fn max_rank_stats() {
        let tab = riven_tab();
        let unranked = tab
            .unveiled
            .iter()
            .find(|row| row.rank < 8 && row.attributes.iter().all(|a| a.display.is_some()))
            .expect("unranked riven");
        let asked = listing_payload(
            unranked,
            &ListingChoices {
                direct: true,
                selling_price: 50,
                rank: 8,
                ..ListingChoices::default()
            },
        )
        .unwrap();
        assert_eq!(asked.item.mod_rank, 8);
        for (attribute, sent) in unranked.attributes.iter().zip(&asked.item.attributes) {
            let shown = attribute.display.unwrap();
            assert!((sent.value - shown).abs() < f64::EPSILON);
        }

        let as_rolled = listing_payload(
            unranked,
            &ListingChoices {
                rank: unranked.rank,
                ..direct(50)
            },
        )
        .unwrap();
        assert_eq!(as_rolled.item.mod_rank, unranked.rank);
        let scale = f64::from(unranked.rank + 1) / 9.0;
        for (attribute, sent) in unranked.attributes.iter().zip(&as_rolled.item.attributes) {
            let rolled = attribute.rolled.unwrap();
            let expected = if attribute.unit.as_deref() == Some("multiply") {
                round_to(rolled * scale + 1.0, 2)
            } else {
                round_to(rolled * scale, 1)
            };
            assert!((sent.value - expected).abs() < f64::EPSILON);
        }
        assert!(
            as_rolled
                .item
                .attributes
                .iter()
                .zip(&asked.item.attributes)
                .any(|(rolled, maxed)| (rolled.value - maxed.value).abs() > f64::EPSILON)
        );
    }

    #[test]
    fn spliced_stat_stays_out_of_the_payload() {
        let fixture = fixture();
        let grader = fixture.grader();
        let tab = riven_tab();
        let cernos = by_weapon(&tab.unveiled, "Proboscis Cernos");
        let riven_type = fixture
            .catalog
            .data()
            .riven_data()
            .riven_type(&cernos.item_type);
        let spliced = RivenFingerprint::parse(&rolled(
            cernos.weapon_path.as_deref().unwrap(),
            &[
                "WeaponCritChanceMod",
                "WeaponFireIterationsMod",
                "WeaponFactionDamageScaldra",
            ],
            Some("WeaponReloadSpeedMod"),
        ))
        .unwrap();
        let attributes = grader.graded(&spliced, riven_type, cernos.disposition);
        let row = RivenRow {
            unlisted_stat: unlisted_stat(&attributes),
            attributes,
            ..cernos.clone()
        };
        let payload = listing_payload(&row, &direct(100)).unwrap();
        assert_eq!(row.unlisted_stat.as_deref(), Some("Damage to Scaldra"));
        assert_eq!(
            payload
                .item
                .attributes
                .iter()
                .map(|attribute| attribute.url_name.as_str())
                .collect::<Vec<&str>>(),
            ["critical_chance", "multishot", "reload_speed"]
        );
        assert_eq!(
            payload
                .item
                .attributes
                .iter()
                .filter(|attribute| !attribute.positive)
                .count(),
            1
        );

        let unknown = RivenRow {
            attributes: row
                .attributes
                .iter()
                .cloned()
                .map(|attribute| AttributeGrade {
                    slug: None,
                    ..attribute
                })
                .collect(),
            ..row.clone()
        };
        assert!(listing_payload(&unknown, &direct(100)).is_none());
    }

    #[test]
    fn polarity_lowercase() {
        let tab = riven_tab();
        let cernos = by_weapon(&tab.unveiled, "Proboscis Cernos");
        assert_eq!(cernos.polarity, Some(Polarity::Madurai));
        let json = serde_json::to_value(cernos).unwrap();
        assert_eq!(json["polarity"], "madurai");
        let payload = listing_payload(cernos, &direct(100)).unwrap();
        assert_eq!(payload.item.polarity, Polarity::Madurai);
    }
}
