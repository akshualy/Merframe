use std::collections::BTreeMap;

use serde::Serialize;
use wf_inventory::Inventory;

use crate::catalog::{Catalog, display_name_from_path, is_resource, owned_count};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceRow {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub owned: i64,
    pub required: i64,
    pub deficit: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecipeDemand {
    pub recipe: String,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourcesTab {
    pub resources: Vec<ResourceRow>,
    pub recipes: Vec<RecipeDemand>,
}

pub(crate) fn tab(inventory: &Inventory, catalog: &Catalog) -> ResourcesTab {
    let mut required: BTreeMap<&str, i64> = BTreeMap::new();
    let mut recipes = Vec::new();
    for recipe in &inventory.recipes {
        if recipe.item_count <= 0 {
            continue;
        }
        let Some((item, component)) = catalog.component_for_reward(&recipe.item_type) else {
            continue;
        };
        if !component.unique_name.ends_with("Blueprint") {
            continue;
        }
        let Some(components) = item.components.as_deref() else {
            continue;
        };
        recipes.push(RecipeDemand {
            recipe: recipe.item_type.clone(),
            name: item.name.clone(),
            count: recipe.item_count,
        });
        for part in components.iter().filter(|part| is_resource(part)) {
            *required.entry(part.unique_name.as_str()).or_insert(0) +=
                i64::from(part.item_count) * recipe.item_count;
        }
    }

    let resources = required
        .into_iter()
        .map(|(unique_name, required)| {
            let owned = owned_count(inventory, unique_name);
            ResourceRow {
                name: resource_name(catalog, unique_name),
                image_name: catalog.icon_for(unique_name),
                unique_name: unique_name.to_owned(),
                owned,
                required,
                deficit: (required - owned).max(0),
            }
        })
        .collect();
    ResourcesTab { resources, recipes }
}

fn resource_name(catalog: &Catalog, unique_name: &str) -> String {
    if let Some((_, component)) = catalog.component(unique_name) {
        return component.name.clone();
    }
    display_name_from_path(unique_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;

    #[test]
    fn no_owned_blueprints() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let tab = tab(&inventory, &catalog);
        assert!(tab.resources.is_empty());
        assert!(tab.recipes.is_empty());
    }

    #[test]
    fn braton_prime_requirements() {
        let json = fixtures::INVENTORY.replacen(
            r#""Recipes":[{"#,
            r#""Recipes":[{"ItemType":"/Lotus/Types/Recipes/Weapons/BratonPrimeBlueprint","ItemCount":3},{"#,
            1,
        );
        let inventory = Inventory::parse(&json).unwrap();
        let catalog = fixtures::catalog();
        let tab = tab(&inventory, &catalog);

        assert_eq!(tab.recipes.len(), 1);
        assert_eq!(tab.recipes[0].name, "Braton Prime");
        assert_eq!(tab.recipes[0].count, 3);

        assert_eq!(tab.resources.len(), 1);
        let cell = &tab.resources[0];
        assert_eq!(cell.name, "Orokin Cell");
        assert_eq!(cell.required, 30);
        assert_eq!(cell.owned, 3319);
        assert_eq!(cell.deficit, 0);

        let with_skins = super::tab(&inventory, &fixtures::with_skins(fixtures::ITEMS));
        assert_eq!(with_skins, tab);
    }

    #[test]
    fn deficit() {
        let json = fixtures::INVENTORY.replacen(
            r#""Recipes":[{"#,
            r#""Recipes":[{"ItemType":"/Lotus/Types/Recipes/Weapons/BratonPrimeBlueprint","ItemCount":1000},{"#,
            1,
        );
        let inventory = Inventory::parse(&json).unwrap();
        let catalog = fixtures::catalog();
        let tab = tab(&inventory, &catalog);
        let cell = &tab.resources[0];
        assert_eq!(cell.required, 10_000);
        assert_eq!(cell.deficit, 10_000 - 3319);
    }
}
