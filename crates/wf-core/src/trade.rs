use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, part_name};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeItem {
    pub name: String,
    pub count: i64,
    pub rank: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Trade {
    pub offered: Vec<TradeItem>,
    pub received: Vec<TradeItem>,
    pub plat: i64,
}

pub(crate) fn parse_trade_description(description: &str) -> Option<(Option<String>, Trade)> {
    let mut lines = description.lines().map(str::trim);
    if !lines
        .next()?
        .trim_end_matches(':')
        .ends_with("accept this trade? You are offering")
    {
        return None;
    }
    let mut trade = Trade::default();
    let mut partner = None;
    let mut receiving = false;
    for line in lines.filter(|line| !line.is_empty()) {
        if let Some(name) = partner_line(line) {
            partner = Some(player_name(name));
            receiving = true;
            continue;
        }
        let item = parse_entry(line);
        if item.name == "Platinum" {
            trade.plat += if receiving { item.count } else { -item.count };
        } else if receiving {
            add_item(&mut trade.received, item);
        } else {
            add_item(&mut trade.offered, item);
        }
    }
    Some((partner, trade))
}

fn partner_line(line: &str) -> Option<&str> {
    line.strip_prefix("and will receive from ")?
        .strip_suffix(" the following:")
}

pub(crate) fn player_name(name: &str) -> String {
    name.trim_end_matches(|character: char| !character.is_ascii())
        .trim()
        .to_owned()
}

fn parse_entry(entry: &str) -> TradeItem {
    let (name, count) = entry
        .rsplit_once(" x ")
        .and_then(|(name, count)| Some((name.trim(), count.trim().parse().ok()?)))
        .unwrap_or((entry, 1));
    let (name, rank) = split_rank(name);
    TradeItem {
        name: plain_name(name).to_owned(),
        count,
        rank,
    }
}

fn add_item(items: &mut Vec<TradeItem>, item: TradeItem) {
    match items
        .iter_mut()
        .find(|known| known.name == item.name && known.rank == item.rank)
    {
        Some(known) => known.count += item.count,
        None => items.push(item),
    }
}

fn plain_name(name: &str) -> &str {
    let name = name.strip_suffix(" Defiled").unwrap_or(name);
    if name.is_ascii() {
        return name;
    }
    name.rsplit_once(' ').map_or(name, |(head, _)| head)
}

pub fn traded_set(catalog: &Catalog, items: &[TradeItem]) -> Option<TradeItem> {
    let (first, rest) = items.split_first()?;
    if rest.is_empty() {
        return None;
    }
    let (set, _) = catalog
        .items()
        .flat_map(|item| {
            item.components
                .iter()
                .flatten()
                .map(move |part| (item, part))
        })
        .find(|(item, part)| same_part(&first.name, &part_name(item, part)))?;
    let parts: Vec<String> = set
        .components
        .iter()
        .flatten()
        .map(|part| part_name(set, part))
        .collect();
    if !items
        .iter()
        .all(|item| parts.iter().any(|part| same_part(&item.name, part)))
    {
        return None;
    }
    let mut count = items.iter().map(|item| item.count).min()?;
    if set.name == "Dual Decurion" {
        count /= 2;
    }
    Some(TradeItem {
        name: format!("{} Set", set.name),
        count,
        rank: None,
    })
}

pub fn same_part(traded: &str, catalog: &str) -> bool {
    without_blueprint(traded).eq_ignore_ascii_case(without_blueprint(catalog))
}

fn without_blueprint(name: &str) -> &str {
    let name = name.strip_suffix(" Blueprint").unwrap_or(name);
    if let Some((base, _)) = relic_refinement(name) {
        return base;
    }
    match name.rsplit_once(" (") {
        Some((base, size)) if size.len() == 2 && size.ends_with(')') => base,
        _ => name,
    }
}

pub fn relic_refinement(name: &str) -> Option<(&str, String)> {
    if let Some((base, refinement)) = name.rsplit_once(" [") {
        let refinement = refinement.strip_suffix(']')?;
        return base
            .ends_with(" Relic")
            .then(|| (base, refinement.to_lowercase()));
    }
    name.ends_with(" Relic")
        .then(|| (name, String::from("intact")))
}

fn split_rank(name: &str) -> (&str, Option<u32>) {
    let Some((base, suffix)) = name.rsplit_once(" (") else {
        return (name, None);
    };
    let Some(rank) = suffix
        .strip_suffix(')')
        .and_then(|suffix| suffix.rsplit_once(" RANK "))
        .and_then(|(_, rank)| rank.parse().ok())
    else {
        return (name, None);
    };
    (base, Some(rank))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_for_platinum() {
        let (partner, trade) = parse_trade_description(
            "Are you sure you want to accept this trade? You are offering\nLoki Prime Systems Blueprint\nForma Blueprint x 2\nand will receive from TestSquadA\u{e000} the following:\nPlatinum x 45\n",
        )
        .unwrap();
        assert_eq!(partner.as_deref(), Some("TestSquadA"));
        assert_eq!(
            trade.offered,
            vec![
                TradeItem {
                    name: String::from("Loki Prime Systems Blueprint"),
                    count: 1,
                    rank: None,
                },
                TradeItem {
                    name: String::from("Forma Blueprint"),
                    count: 2,
                    rank: None,
                },
            ]
        );
        assert!(trade.received.is_empty());
        assert_eq!(trade.plat, 45);
    }

    #[test]
    fn platinum_for_items() {
        let (partner, trade) = parse_trade_description(
            "Are you sure you want to accept this trade? You are offering\nPlatinum x 120\nand will receive from TestSquadB the following:\nRhino Prime Blueprint\nMesa Prime Chassis Blueprint\n",
        )
        .unwrap();
        assert_eq!(partner.as_deref(), Some("TestSquadB"));
        assert_eq!(trade.plat, -120);
        assert!(trade.offered.is_empty());
        assert_eq!(trade.received.len(), 2);
        assert_eq!(trade.received[1].name, "Mesa Prime Chassis Blueprint");
    }

    #[test]
    fn game_block_with_colon_ranks_and_fish_sizes() {
        let (partner, trade) = parse_trade_description(
            "Are you sure you want to accept this trade? You are offering:\nGoopolla (L)\nGoopolla (M)\nPlatinum x 12\nGoopolla (S)\n\nand will receive from TestSquadA the following:\nNoctua Swarm (RARE RANK 0)",
        )
        .unwrap();
        assert_eq!(partner.as_deref(), Some("TestSquadA"));
        assert_eq!(trade.plat, -12);
        assert_eq!(
            trade
                .offered
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["Goopolla (L)", "Goopolla (M)", "Goopolla (S)"]
        );
        assert_eq!(
            trade.received,
            [TradeItem {
                name: String::from("Noctua Swarm"),
                count: 1,
                rank: Some(0),
            }]
        );
    }

    #[test]
    fn repeated_lines_add_up() {
        let (_, trade) = parse_trade_description(
            "Are you sure you want to accept this trade? You are offering:\nGoopolla (S)\nGoopolla (S)\nGoopolla (S)\n\nand will receive from TestSquadA the following:\nPlatinum x 10",
        )
        .unwrap();
        assert_eq!(
            trade.offered,
            [TradeItem {
                name: String::from("Goopolla (S)"),
                count: 3,
                rank: None,
            }]
        );
        assert!(same_part("Goopolla (S)", "Goopolla"));
    }

    #[test]
    fn relic_refinements() {
        assert_eq!(
            relic_refinement("Axi D6 Relic [EXCEPTIONAL]"),
            Some(("Axi D6 Relic", String::from("exceptional")))
        );
        assert_eq!(
            relic_refinement("Axi A5 Relic"),
            Some(("Axi A5 Relic", String::from("intact")))
        );
        assert_eq!(relic_refinement("Forma Blueprint"), None);
        assert!(same_part("Lith Q3 Relic [RADIANT]", "Lith Q3 Relic"));
    }

    #[test]
    fn riven_rank_suffix() {
        assert_eq!(
            split_rank("Rubico Critacan (RIVEN RANK 8)"),
            ("Rubico Critacan", Some(8))
        );
        assert_eq!(
            split_rank("Ayatan Anasa Sculpture"),
            ("Ayatan Anasa Sculpture", None)
        );
    }

    #[test]
    fn defiled_and_arcane_rank_glyphs_are_dropped() {
        assert_eq!(
            parse_entry("Primed Continuity Defiled (RARE RANK 3)").name,
            "Primed Continuity"
        );
        assert_eq!(
            parse_entry("Arcane Grace \u{e001}\u{e001}\u{e001}").name,
            "Arcane Grace"
        );
        assert_eq!(
            parse_entry("Okina Acri-Vexicak (RIVEN RANK 8)").rank,
            Some(8)
        );
    }

    #[test]
    fn full_set_collapses() {
        let catalog = crate::catalog::fixtures::catalog();
        let item = |name: &str, count: i64| TradeItem {
            name: name.to_owned(),
            count,
            rank: None,
        };
        let parts = [
            item("Braton Prime Barrel", 2),
            item("Braton Prime Receiver", 1),
            item("Braton Prime Stock", 1),
            item("Braton Prime Blueprint", 1),
        ];
        assert_eq!(
            traded_set(&catalog, &parts),
            Some(item("Braton Prime Set", 1))
        );
        assert_eq!(traded_set(&catalog, &parts[..1]), None);
        let mixed = [item("Braton Prime Barrel", 1), item("Forma Blueprint", 1)];
        assert_eq!(traded_set(&catalog, &mixed), None);
    }

    #[test]
    fn dialog_head_only() {
        let (partner, trade) =
            parse_trade_description("Are you sure you want to accept this trade? You are offering")
                .unwrap();
        assert_eq!(partner, None);
        assert_eq!(trade, Trade::default());
    }

    #[test]
    fn other_dialogs() {
        assert!(parse_trade_description("Are you sure you want to accept this trade?").is_none());
        assert!(
            parse_trade_description("Are you sure you want to equip Lith K12 Relic?").is_none()
        );
    }
}
