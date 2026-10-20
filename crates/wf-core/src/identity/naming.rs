use wf_data::{Component, Item};

use crate::catalog::part_name;

pub fn market_slug(display_name: &str) -> String {
    let mut slug = String::with_capacity(display_name.len());
    let mut pending_separator = false;
    for ch in display_name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('_');
            }
            pending_separator = false;
            slug.push(ch.to_ascii_lowercase());
        } else if ch != '\'' {
            pending_separator = true;
        }
    }
    slug
}

pub(crate) fn part_market_slug(item: &Item, component: &Component) -> String {
    let name = part_name(item, component);
    if component.unique_name.ends_with("Component") {
        market_slug(&format!("{name} Blueprint"))
    } else {
        market_slug(&name)
    }
}

const LANDING_CRAFT_RECIPES: &str = "/Lotus/Types/Recipes/LandingCraftRecipes/";

pub(crate) fn is_landing_craft_part(unique_name: &str) -> bool {
    unique_name.starts_with(LANDING_CRAFT_RECIPES)
}

/// WFCD flags every landing craft part untradable, but the Avionics, Engines and Fuselage
/// blueprints of every landing craft are listed on warframe.market.
pub(crate) fn part_tradable(component: &Component) -> bool {
    component.tradable
        || (is_landing_craft_part(&component.unique_name) && component.name != "Blueprint")
}

pub(crate) fn blueprint_of(component: &str) -> Option<String> {
    component
        .strip_suffix("Component")
        .map(|base| format!("{base}Blueprint"))
}

pub(crate) fn ambassador_blueprint(unique_name: &str) -> Option<String> {
    if !unique_name.contains("CrpArSniper") || unique_name.contains("Blueprint") {
        return None;
    }
    Some(format!(
        "{}Blueprint",
        unique_name.replace("CrpArSniper", "Ambassador")
    ))
}

pub(crate) fn held_names(component: &str) -> impl Iterator<Item = String> {
    [
        Some(format!("{component}Blueprint")),
        blueprint_of(component),
    ]
    .into_iter()
    .flatten()
}

pub(crate) fn component_of(blueprint: &str) -> Option<String> {
    blueprint
        .strip_suffix("Blueprint")
        .map(|base| format!("{base}Component"))
}

pub(crate) fn part_identity(unique_name: &str) -> &str {
    unique_name
        .strip_suffix("Blueprint")
        .or_else(|| unique_name.strip_suffix("Component"))
        .unwrap_or(unique_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures::{TRINITY_PRIME_SYSTEMS, TRINITY_PRIME_SYSTEMS_BLUEPRINT};

    #[test]
    fn component_blueprint_aliases() {
        assert_eq!(
            blueprint_of(TRINITY_PRIME_SYSTEMS).as_deref(),
            Some(TRINITY_PRIME_SYSTEMS_BLUEPRINT)
        );
        assert_eq!(
            component_of(TRINITY_PRIME_SYSTEMS_BLUEPRINT).as_deref(),
            Some(TRINITY_PRIME_SYSTEMS)
        );
        assert_eq!(blueprint_of(TRINITY_PRIME_SYSTEMS_BLUEPRINT), None);
        assert_eq!(component_of(TRINITY_PRIME_SYSTEMS), None);
        assert_eq!(
            part_identity(TRINITY_PRIME_SYSTEMS),
            part_identity(TRINITY_PRIME_SYSTEMS_BLUEPRINT)
        );
    }

    #[test]
    fn slug_rules() {
        assert_eq!(market_slug("Braton Prime Barrel"), "braton_prime_barrel");
        assert_eq!(market_slug("Axi A1 Relic"), "axi_a1_relic");
        assert_eq!(market_slug("Gaia's Tragedy"), "gaias_tragedy");
        assert_eq!(
            market_slug("Zephyr Prime  Blueprint"),
            "zephyr_prime_blueprint"
        );
    }

    #[test]
    fn landing_craft_parts_trade() {
        let untraded = |unique_name: &str, name: &str| Component {
            unique_name: unique_name.to_owned(),
            name: name.to_owned(),
            item_count: 1,
            tradable: false,
            ducats: None,
            drops: None,
            image_name: None,
        };
        let parallax = "/Lotus/Types/Recipes/LandingCraftRecipes/ZarimanShip/";
        assert!(part_tradable(&untraded(
            &format!("{parallax}ZarimanShipAvionicsComponent"),
            "Avionics"
        )));
        assert!(!part_tradable(&untraded(
            &format!("{parallax}ZarimanShipBlueprint"),
            "Blueprint"
        )));
        assert!(!part_tradable(&untraded(TRINITY_PRIME_SYSTEMS, "Systems")));
    }
}
