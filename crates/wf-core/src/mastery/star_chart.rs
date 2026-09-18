use std::collections::HashSet;

use wf_inventory::Inventory;

use super::routes::member_route;
use super::{CategoryTotals, LevelUpRoute, RouteMember};

type ClearedNodes<'a> = (HashSet<&'a str>, HashSet<&'a str>);

fn is_junction(tag: &str) -> bool {
    tag.ends_with("Junction") && tag.contains("To")
}

fn beaten_junctions(inventory: &Inventory) -> ClearedNodes<'_> {
    let entries = inventory
        .missions
        .iter()
        .filter(|mission| is_junction(&mission.tag) && mission.completes > 0);
    let mut normal = HashSet::new();
    let mut steel = HashSet::new();
    for mission in entries {
        normal.insert(mission.tag.as_str());
        if mission.completes == 2 || (mission.completes >= 1 && mission.tier.unwrap_or(0) >= 1) {
            steel.insert(mission.tag.as_str());
        }
    }
    (normal, steel)
}

fn cleared_nodes(inventory: &Inventory) -> ClearedNodes<'_> {
    let mut normal = HashSet::new();
    let mut steel = HashSet::new();
    for mission in &inventory.missions {
        if !wf_worldstate::is_masterable_node(&mission.tag) {
            continue;
        }
        normal.insert(mission.tag.as_str());
        if mission.tier.unwrap_or(0) > 0 {
            steel.insert(mission.tag.as_str());
        }
    }
    (normal, steel)
}

fn cleared_node_xp(cleared: &HashSet<&str>) -> u64 {
    cleared
        .iter()
        .map(|node_id| u64::from(wf_worldstate::node_mastery_xp(node_id)))
        .sum()
}

pub(super) fn star_chart(
    inventory: &Inventory,
) -> (
    CategoryTotals,
    CategoryTotals,
    CategoryTotals,
    CategoryTotals,
) {
    let total_nodes = wf_worldstate::masterable_node_count();
    let total_junctions = wf_worldstate::junction_count();
    let (normal, steel) = cleared_nodes(inventory);
    let (junction_normal, junction_steel) = beaten_junctions(inventory);
    (
        CategoryTotals::counted(normal.len(), total_nodes),
        CategoryTotals::counted(steel.len(), total_nodes),
        CategoryTotals::counted(junction_normal.len(), total_junctions),
        CategoryTotals::counted(junction_steel.len(), total_junctions),
    )
}

pub(super) fn star_chart_xp(inventory: &Inventory) -> u64 {
    let (normal, steel) = cleared_nodes(inventory);
    let (junction_normal, junction_steel) = beaten_junctions(inventory);
    let per_junction = u64::from(wf_worldstate::junction_mastery_xp());
    cleared_node_xp(&normal)
        + cleared_node_xp(&steel)
        + per_junction * (junction_normal.len() + junction_steel.len()) as u64
}

fn node_place(node_id: &str) -> String {
    let Some(info) = wf_worldstate::node_info(node_id) else {
        return node_id.to_owned();
    };
    let value = info.value.trim();
    match value
        .strip_suffix(')')
        .and_then(|inner| inner.rsplit_once('('))
    {
        Some((name, planet)) => format!("{}, {}", planet.trim(), name.trim()),
        None => value.to_owned(),
    }
}

fn node_member(node_id: &str, xp: u64) -> RouteMember {
    let place = node_place(node_id);
    let kind = wf_worldstate::node_info(node_id).map_or("", |info| info.kind);
    RouteMember {
        name: if kind.is_empty() {
            place
        } else {
            format!("{place} ({kind})")
        },
        detail: String::new(),
        xp,
    }
}

fn missing_nodes(cleared: &HashSet<&str>) -> Vec<RouteMember> {
    wf_worldstate::masterable_nodes()
        .filter(|(node_id, _)| !cleared.contains(node_id))
        .map(|(node_id, xp)| node_member(node_id, u64::from(xp)))
        .collect()
}

fn missing_junctions(beaten: &HashSet<&str>, per_junction: u64) -> Vec<RouteMember> {
    wf_worldstate::junction_nodes()
        .filter(|node_id| !beaten.contains(node_id))
        .map(|node_id| RouteMember {
            name: node_place(node_id),
            detail: String::new(),
            xp: per_junction,
        })
        .collect()
}

pub(super) fn routes(inventory: &Inventory) -> [LevelUpRoute; 4] {
    let (cleared_normal, cleared_steel) = cleared_nodes(inventory);
    let normal = missing_nodes(&cleared_normal);
    let steel = missing_nodes(&cleared_steel);
    let (beaten_normal, beaten_steel) = beaten_junctions(inventory);
    let per_junction = u64::from(wf_worldstate::junction_mastery_xp());
    let junctions = missing_junctions(&beaten_normal, per_junction);
    let steel_junctions = missing_junctions(&beaten_steel, per_junction);
    [
        member_route(
            "star_chart",
            "Clear the rest of the star chart",
            "node",
            normal,
        ),
        member_route(
            "steel_path",
            "Clear the star chart on the Steel Path",
            "node",
            steel,
        ),
        member_route(
            "junctions",
            "Beat the junction specters",
            "junction",
            junctions,
        ),
        member_route(
            "steel_path_junctions",
            "Beat the junctions on the Steel Path",
            "junction",
            steel_junctions,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;

    #[test]
    fn complete_star_chart() {
        let inventory = fixtures::inventory();
        let (normal, steel, junction, steel_junction) = star_chart(&inventory);
        assert_eq!(normal.current, 169);
        assert_eq!(normal.max, 169);
        assert_eq!(steel.current, 169);
        assert_eq!(steel.max, 169);
        assert_eq!(junction.current, 13);
        assert_eq!(junction.max, 13);
        assert_eq!(steel_junction.current, 13);
        assert_eq!(steel_junction.max, 13);
        assert_eq!(star_chart_xp(&inventory), 55_138);
    }
}
