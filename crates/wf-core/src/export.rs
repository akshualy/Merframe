use std::path::Path;

use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::foundry::FoundryTab;
use crate::inventory_view::InventoryTab;
use crate::rivens::RivensTab;

pub(crate) struct ExportBundle<'a> {
    pub document: &'a str,
    pub inventory: &'a InventoryTab,
    pub rivens: &'a RivensTab,
    pub foundry: &'a FoundryTab,
}

pub(crate) fn export(dir: &Path, bundle: &ExportBundle<'_>) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|source| CoreError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let path = dir.join("inventory.json");
    std::fs::write(&path, bundle.document).map_err(|source| CoreError::Io { path, source })?;
    write(dir, "parts.json", &bundle.inventory.parts)?;
    write(dir, "mods.json", &bundle.inventory.mods)?;
    write(dir, "arcanes.json", &bundle.inventory.arcanes)?;
    write(dir, "relics.json", &bundle.inventory.relics)?;
    write(dir, "misc.json", &bundle.inventory.misc)?;
    write(dir, "sets.json", &bundle.inventory.sets)?;
    write(dir, "rivens.json", bundle.rivens)?;
    write(dir, "foundry.json", bundle.foundry)
}

fn write<T: Serialize>(dir: &Path, name: &str, value: &T) -> Result<()> {
    let path = dir.join(name);
    let json = serde_json::to_vec_pretty(value)?;
    std::fs::write(&path, json).map_err(|source| CoreError::Io { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::favourites::Favourites;
    use crate::listings::MarketListings;
    use crate::prices::FixedPrices;
    use crate::view::View;
    use crate::{foundry, inventory_view, rivens};

    #[test]
    fn one_file_per_tab() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let prices = FixedPrices::default();
        let favourites = Favourites::default();
        let listings = MarketListings::default();
        let inventory_tab = inventory_view::tab(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices,
            favourites: &favourites,
            listings: &listings,
        });
        let rivens_tab = rivens::Grader::new(&catalog, &[], None).tab(&inventory, &listings);
        let now = chrono::DateTime::from_timestamp_millis(1_788_807_069_000).unwrap();
        let foundry_tab = foundry::tab(&inventory, &catalog, None, None, now, &favourites);

        let dir = std::env::temp_dir().join("wf-core-export-test");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove dir");
        }
        export(
            &dir,
            &ExportBundle {
                document: fixtures::INVENTORY,
                inventory: &inventory_tab,
                rivens: &rivens_tab,
                foundry: &foundry_tab,
            },
        )
        .unwrap();

        for name in [
            "parts.json",
            "mods.json",
            "arcanes.json",
            "relics.json",
            "misc.json",
            "sets.json",
            "rivens.json",
            "foundry.json",
        ] {
            let path = dir.join(name);
            let body = std::fs::read_to_string(&path).unwrap();
            assert!(
                serde_json::from_str::<serde_json::Value>(&body).is_ok(),
                "{name} is not valid json"
            );
        }

        assert_eq!(
            std::fs::read_to_string(dir.join("inventory.json")).unwrap(),
            fixtures::INVENTORY
        );
        let relics: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("relics.json")).unwrap())
                .unwrap();
        assert_eq!(relics.len(), 26);
        std::fs::remove_dir_all(&dir).expect("remove dir");
    }
}
