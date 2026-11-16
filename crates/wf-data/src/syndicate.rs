use std::collections::HashMap;

use serde::Deserialize;

use crate::dictionary::Dictionary;
use crate::error::{DataError, Result};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSyndicate {
    name: String,
    daily_limit_bin: String,
    #[serde(default)]
    alignments: HashMap<String, f64>,
    #[serde(default)]
    titles: Vec<SyndicateTitle>,
    favours: Vec<SyndicateOffer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SyndicateSacrifice {
    pub credits: u32,
    pub items: Vec<SyndicateSacrificeItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SyndicateSacrificeItem {
    pub item_type: String,
    pub item_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyndicateOffer {
    pub store_item: String,
    pub standing_cost: u32,
    pub required_level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyndicateTitle {
    pub level: i32,
    pub name: String,
    pub min_standing: i64,
    pub max_standing: i64,
    pub sacrifice: Option<SyndicateSacrifice>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Syndicate {
    pub tag: String,
    pub name: String,
    pub daily_limit_bin: String,
    pub alignments: HashMap<String, f64>,
    pub titles: Vec<SyndicateTitle>,
    pub offers: Vec<SyndicateOffer>,
}

fn title_name(text: &str) -> String {
    if text.chars().any(char::is_lowercase) {
        return text.to_owned();
    }

    text.split(' ')
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or_default(|first| {
                first
                    .to_uppercase()
                    .chain(letters.flat_map(char::to_lowercase))
                    .collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn syndicate(tag: String, raw: RawSyndicate, dictionary: &Dictionary) -> Option<Syndicate> {
    let mut titles = raw.titles;
    for title in &mut titles {
        title.name = title_name(dictionary.get(&title.name)?);
    }
    titles.sort_by_key(|title| title.level);

    Some(Syndicate {
        name: dictionary.get(&raw.name)?.to_owned(),
        tag,
        daily_limit_bin: raw.daily_limit_bin,
        alignments: raw.alignments,
        titles,
        offers: raw.favours,
    })
}

fn is_nightwave_season(tag: &str) -> bool {
    tag.starts_with("RadioLegion")
}

pub fn parse_syndicates(export: &str, dictionary: &Dictionary) -> Result<Vec<Syndicate>> {
    let export: HashMap<String, RawSyndicate> =
        serde_json::from_str(export).map_err(|source| DataError::Parse("syndicate", source))?;

    Ok(export
        .into_iter()
        .filter(|(tag, _)| !is_nightwave_season(tag))
        .filter_map(|(tag, raw)| syndicate(tag, raw, dictionary))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &str = include_str!("../tests/fixtures/syndicates.json");
    const DICTIONARY: &str = include_str!("../tests/fixtures/syndicate_dictionary.json");

    fn syndicates() -> Vec<Syndicate> {
        parse_syndicates(EXPORT, &Dictionary::from_json(DICTIONARY).unwrap()).unwrap()
    }

    fn syndicate(tag: &str) -> Syndicate {
        syndicates()
            .into_iter()
            .find(|syndicate| syndicate.tag == tag)
            .unwrap()
    }

    #[test]
    fn nightwave_seasons_left_out() {
        assert!(
            syndicates()
                .iter()
                .all(|syndicate| syndicate.tag != "RadioLegionSyndicate")
        );
    }

    #[test]
    fn titles_by_level() {
        let meridian = syndicate("SteelMeridianSyndicate");

        assert_eq!(
            meridian
                .titles
                .iter()
                .map(|title| title.level)
                .collect::<Vec<_>>(),
            [-2, -1, 1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn shouted_titles_in_title_case() {
        assert_eq!(
            syndicate("VentKidsSyndicate").titles[2].name,
            "Proper Felon"
        );
        assert_eq!(title_name("CLEARANCE: AGNESIS"), "Clearance: Agnesis");
        assert_eq!(title_name("2-for-1"), "2-for-1");
    }

    #[test]
    fn untranslated_syndicate_left_out() {
        let export = r#"{"NewSyndicate":{"name":"/Lotus/Language/Syndicates/NewName","dailyLimitBin":"STANDING_LIMIT_BIN_NONE","favours":[]}}"#;

        assert!(
            parse_syndicates(export, &Dictionary::default())
                .unwrap()
                .is_empty()
        );
    }
}
