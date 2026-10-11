use wf_data::{Component, Item, catch_grade, misc_item_name, store_item_to_type};

use super::unlisted_upgrade;
use crate::catalog::{Catalog, display_name_from_path, part_name, refinement_name};

pub(crate) fn market_slug(display_name: &str) -> String {
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

pub(crate) fn display_name(catalog: &Catalog, item_type: &str) -> String {
    let unique_name = store_item_to_type(item_type);
    catalogued_name(catalog, &unique_name).unwrap_or_else(|| display_name_from_path(&unique_name))
}

pub(crate) fn catalogued_name(catalog: &Catalog, unique_name: &str) -> Option<String> {
    if let Some((relic, refinement)) = catalog.relic_by_unique_name(unique_name) {
        return Some(format!(
            "{} Relic ({})",
            relic.name,
            refinement_name(refinement)
        ));
    }

    if let Some((base, grade)) = catch_grade(unique_name)
        && let Some(item) = catalog.item(&base)
    {
        return Some(format!("{} ({grade})", item.name));
    }

    if let Some(item) = catalog.item(unique_name) {
        return Some(item.name.clone());
    }

    if let Some((item, component)) = catalog.component(unique_name) {
        return Some(part_name(item, component));
    }

    if let Some(upgrade) = unlisted_upgrade(unique_name) {
        return Some(upgrade.name.to_owned());
    }

    misc_item_name(unique_name).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
