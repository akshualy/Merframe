pub use crate::embedded::SolNode;
use crate::embedded::{NODE_MASTERY, SOL_NODES, lookup};

fn is_junction(node_id: &str) -> bool {
    node_id.ends_with("Junction")
}

pub fn node_info(node_id: &str) -> Option<&'static SolNode> {
    lookup(SOL_NODES, node_id)
}

pub fn node_name(node_id: &str) -> Option<&'static str> {
    node_info(node_id).map(|node| node.value)
}

pub fn node_mastery_xp(node_id: &str) -> u32 {
    lookup(NODE_MASTERY, node_id).copied().unwrap_or_default()
}

pub fn is_masterable_node(node_id: &str) -> bool {
    !is_junction(node_id) && node_mastery_xp(node_id) > 0
}

pub fn masterable_nodes() -> impl Iterator<Item = (&'static str, u32)> {
    NODE_MASTERY
        .iter()
        .filter(|(node_id, xp)| *xp > 0 && !is_junction(node_id))
        .map(|(node_id, xp)| (*node_id, *xp))
}

pub fn junction_nodes() -> impl Iterator<Item = &'static str> {
    NODE_MASTERY
        .iter()
        .map(|(node_id, _)| *node_id)
        .filter(|node_id| is_junction(node_id))
}

pub fn masterable_node_count() -> usize {
    masterable_nodes().count()
}

pub fn masterable_node_xp_total() -> u32 {
    masterable_nodes().map(|(_, xp)| xp).sum()
}

pub fn junction_mastery_xp() -> u32 {
    NODE_MASTERY
        .iter()
        .filter(|(node_id, _)| is_junction(node_id))
        .map(|(_, xp)| *xp)
        .max()
        .unwrap_or_default()
}

pub fn junction_count() -> usize {
    junction_nodes().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_tables_sorted() {
        assert!(SOL_NODES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(NODE_MASTERY.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn masterable_node_table() {
        assert!(is_masterable_node("SolNode27"));
        assert!(!is_masterable_node("EarthToVenusJunction"));
        assert!(!is_masterable_node("ClanNode0"));
        assert!(!is_masterable_node("NotARealNode"));
        assert!(!is_masterable_node("SolNode850"));
        assert_eq!(masterable_node_count(), 169);
        assert_eq!(masterable_node_xp_total(), 14_569);
        assert_eq!(masterable_nodes().count(), 169);
        assert!(masterable_nodes().all(|(node_id, xp)| xp > 0 && node_name(node_id).is_some()));
    }

    #[test]
    fn junction_count_and_xp() {
        assert_eq!(junction_mastery_xp(), 1000);
        assert_eq!(junction_count(), 13);
        assert!(junction_nodes().all(|node_id| node_name(node_id).is_some()));
        assert!(junction_nodes().all(|node_id| node_mastery_xp(node_id) == 1000));
        assert_eq!(node_mastery_xp("EarthToVenusJunction"), 1000);
    }

    #[test]
    fn node_xp() {
        assert_eq!(node_mastery_xp("SolNode27"), 24);
        assert_eq!(node_mastery_xp("SolNode223"), 3);
        assert_eq!(node_mastery_xp("SolNode108"), 25);
        assert_eq!(node_mastery_xp("SolNode104"), 41);
        assert_eq!(node_mastery_xp("SolNode28"), 0);
        assert_eq!(node_mastery_xp("NotARealNode"), 0);
    }

    #[test]
    fn known_node_names() {
        assert_eq!(node_name("SolNode1"), Some("Galatea (Neptune)"));
        assert_eq!(
            node_name("EarthToVenusJunction"),
            Some("Venus Junction (Earth)")
        );
        assert_eq!(node_name("SolNode850"), Some("Köbinn West (Höllvania)"));
        assert_eq!(node_name("CetusHub4"), Some("Cetus (Earth)"));
    }

    #[test]
    fn unknown_node_is_none() {
        assert_eq!(node_name("NotARealNode"), None);
    }
}
