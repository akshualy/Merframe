use serde::Deserialize;
use wf_data::{MAX_RANK, rank_multiplier};
use wf_market::{CreateAuctionItem, CreateAuctionRequest, RivenAttributeInstance};

use super::RivenRow;
use super::grading::display_value;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingChoices {
    pub direct: bool,
    pub selling_price: u32,
    pub starting_price: u32,
    pub buyout_price: u32,
    pub min_reputation: u32,
    pub note: Option<String>,
    pub private: bool,
    pub max_rank_stats: bool,
}

pub fn listing_payload(row: &RivenRow, choices: &ListingChoices) -> Option<CreateAuctionRequest> {
    let mut attributes = Vec::with_capacity(row.attributes.len());
    for attribute in &row.attributes {
        let value = if choices.max_rank_stats {
            attribute.display?
        } else {
            display_value(
                attribute.rolled? * rank_multiplier(row.rank),
                attribute.unit.as_deref() == Some("multiply"),
            )
        };
        attributes.push(RivenAttributeInstance {
            value,
            positive: !attribute.curse,
            url_name: attribute.slug.clone()?,
        });
    }
    let asking = if choices.direct {
        choices.selling_price
    } else {
        choices.buyout_price
    };
    Some(CreateAuctionRequest {
        item: CreateAuctionItem {
            attributes,
            polarity: row.polarity?,
            mod_rank: if choices.max_rank_stats {
                MAX_RANK
            } else {
                row.rank
            },
            name: row.name.clone()?,
            re_rolls: row.rerolls,
            mastery_level: row.rank_required?,
            weapon_url_name: row.weapon_slug.clone()?,
        },
        buyout_price: (asking > 0).then_some(asking),
        starting_price: if choices.direct {
            choices.selling_price
        } else {
            choices.starting_price
        },
        minimal_reputation: Some(if choices.direct {
            0
        } else {
            choices.min_reputation
        }),
        note: choices.note.clone(),
        private: choices.private,
        visible: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rivens::grading::round_to;
    use crate::rivens::support::*;
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
            note: Some("merframe".to_owned()),
            private: true,
            ..ListingChoices::default()
        };
        let payload = listing_payload(cernos, &auction).unwrap();
        assert_eq!(payload.starting_price, 200);
        assert_eq!(payload.buyout_price, Some(400));
        assert_eq!(payload.minimal_reputation, Some(7));
        assert_eq!(payload.note.as_deref(), Some("merframe"));
        assert!(payload.private);
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
    }

    #[test]
    fn direct_sale_payload() {
        let tab = riven_tab();
        let cernos = by_weapon(&tab.unveiled, "Proboscis Cernos");
        let payload = listing_payload(cernos, &direct(120)).unwrap();
        assert_eq!(payload.starting_price, 120);
        assert_eq!(payload.buyout_price, Some(120));
        assert_eq!(payload.minimal_reputation, Some(0));
        assert!(!payload.private);

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
                max_rank_stats: true,
                ..ListingChoices::default()
            },
        )
        .unwrap();
        assert_eq!(asked.item.mod_rank, 8);
        for (attribute, sent) in unranked.attributes.iter().zip(&asked.item.attributes) {
            let shown = attribute.display.unwrap();
            assert!((sent.value - shown).abs() < f64::EPSILON);
        }

        let as_rolled = listing_payload(unranked, &direct(50)).unwrap();
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
