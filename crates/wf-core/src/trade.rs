use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeItem {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Trade {
    pub offered: Vec<TradeItem>,
    pub received: Vec<TradeItem>,
    pub plat: i64,
}

pub(crate) fn parse_trade_description(description: &str) -> Option<Trade> {
    let (_, body) = description.split_once("accept this trade?")?;
    let body = body.trim().strip_prefix("You are offering ")?;
    let (offered_text, received_text) = body.split_once(" in exchange for ")?;
    let (offered, offered_plat) = parse_side(offered_text);
    let (received, received_plat) = parse_side(received_text);
    Some(Trade {
        offered,
        received,
        plat: received_plat - offered_plat,
    })
}

fn parse_side(text: &str) -> (Vec<TradeItem>, i64) {
    let mut items = Vec::new();
    let mut platinum = 0;
    for entry in split_entries(text) {
        let item = parse_entry(entry);
        if item.name == "Platinum" {
            platinum += item.count;
        } else {
            items.push(item);
        }
    }
    (items, platinum)
}

fn split_entries(text: &str) -> impl Iterator<Item = &str> {
    text.trim()
        .trim_end_matches('.')
        .split(',')
        .flat_map(|part| part.split(" and "))
        .map(str::trim)
        .filter(|part| !part.is_empty())
}

fn parse_entry(entry: &str) -> TradeItem {
    if let Some((count, rest)) = split_leading_count(entry) {
        return TradeItem {
            name: rest.to_owned(),
            count,
        };
    }
    if let Some((name, count)) = split_trailing_count(entry) {
        return TradeItem {
            name: name.to_owned(),
            count,
        };
    }
    TradeItem {
        name: entry.to_owned(),
        count: 1,
    }
}

fn split_leading_count(entry: &str) -> Option<(i64, &str)> {
    let digits = entry
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(entry.len());
    let count: i64 = entry[..digits].parse().ok()?;
    let rest = entry[digits..].trim_start();
    let rest = rest.strip_prefix('x').unwrap_or(rest).trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((count, rest))
}

fn split_trailing_count(entry: &str) -> Option<(&str, i64)> {
    let (name, tail) = entry.rsplit_once(" x")?;
    let count: i64 = tail.trim().parse().ok()?;
    Some((name.trim(), count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_for_platinum() {
        let trade = parse_trade_description("Are you sure you want to accept this trade? You are offering Loki Prime Systems Blueprint, 2 x Forma Blueprint in exchange for 45 Platinum.").unwrap();
        assert_eq!(
            trade.offered,
            vec![
                TradeItem {
                    name: String::from("Loki Prime Systems Blueprint"),
                    count: 1
                },
                TradeItem {
                    name: String::from("Forma Blueprint"),
                    count: 2
                },
            ]
        );
        assert!(trade.received.is_empty());
        assert_eq!(trade.plat, 45);
    }

    #[test]
    fn platinum_for_items() {
        let trade = parse_trade_description(
            "Are you sure you want to accept this trade? You are offering 120 Platinum in exchange for Rhino Prime Blueprint and Mesa Prime Chassis Blueprint.",
        )
        .unwrap();
        assert_eq!(trade.plat, -120);
        assert!(trade.offered.is_empty());
        assert_eq!(trade.received.len(), 2);
        assert_eq!(trade.received[1].name, "Mesa Prime Chassis Blueprint");
    }

    #[test]
    fn trailing_count_form() {
        let trade = parse_trade_description(
            "Are you sure you want to accept this trade? You are offering Ayatan Anasa Sculpture x3 in exchange for 60 Platinum.",
        )
        .unwrap();
        assert_eq!(trade.offered[0].count, 3);
        assert_eq!(trade.offered[0].name, "Ayatan Anasa Sculpture");
    }

    #[test]
    fn dialog_without_offer() {
        assert!(parse_trade_description("Are you sure you want to accept this trade?").is_none());
        assert!(
            parse_trade_description("Are you sure you want to equip Lith K12 Relic?").is_none()
        );
    }
}
