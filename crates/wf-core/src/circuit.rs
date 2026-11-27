use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use wf_data::Item;
use wf_worldstate::Circuit;

use crate::account::{Account, WorldOfferOwnership};
use crate::catalog::Catalog;
use crate::text::contains_word;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct CircuitReward {
    pub name: String,
    pub ownership: Option<WorldOfferOwnership>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct CircuitRotation {
    pub rotates: DateTime<Utc>,
    pub normal: Vec<CircuitReward>,
    pub hard: Vec<CircuitReward>,
}

struct CircuitChoices<'a> {
    by_choice: HashMap<String, &'a Item>,
    adapted: Vec<&'a Item>,
    catalog: &'a Catalog,
    account: Option<&'a Account>,
}

impl CircuitChoices<'_> {
    fn warframe_reward(&self, choice: String) -> CircuitReward {
        let Some(item) = self.by_choice.get(&choice) else {
            return CircuitReward {
                name: choice,
                ownership: None,
            };
        };

        CircuitReward {
            name: item.name.clone(),
            ownership: self
                .account
                .map(|account| account.world_offer_ownership(&item.unique_name, self.catalog)),
        }
    }

    fn incarnon_reward(&self, choice: String) -> CircuitReward {
        let adapter = self.by_choice.get(&format!("{choice}IncarnonGenesis"));
        let name = match self.by_choice.get(&choice) {
            Some(weapon) => weapon.name.clone(),
            None => choice,
        };

        let ownership = self.account.map(|account| {
            let held = adapter.is_some_and(|adapter| account.owns(&adapter.unique_name));
            let installed = self
                .adapted
                .iter()
                .any(|weapon| contains_word(&weapon.name, &name));
            if held || installed {
                WorldOfferOwnership::Owned
            } else {
                WorldOfferOwnership::Missing
            }
        });
        CircuitReward { name, ownership }
    }
}

pub(crate) fn circuit_rotation(
    circuit: Circuit,
    catalog: &Catalog,
    account: Option<&Account>,
) -> CircuitRotation {
    let choices = CircuitChoices {
        by_choice: catalog
            .items()
            .map(|item| (item.name.replace(' ', "").replace('&', "And"), item))
            .collect(),
        adapted: account.map_or_default(|account| {
            account
                .inventory
                .adapted_incarnons()
                .into_iter()
                .filter_map(|item_type| catalog.item(item_type))
                .collect()
        }),
        catalog,
        account,
    };
    CircuitRotation {
        rotates: circuit.rotates,
        normal: circuit
            .normal
            .into_iter()
            .map(|choice| choices.warframe_reward(choice))
            .collect(),
        hard: circuit
            .hard
            .into_iter()
            .map(|choice| choices.incarnon_reward(choice))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::catalog::fixtures::{entries, inventory, inventory_with};
    use crate::identity::ItemTable;

    const CATALOG: &str = r#"[
        {"uniqueName":"/Lotus/Powersuits/Loki/LokiPrime","name":"Loki Prime",
         "category":"Warframes","type":"Warframe","tradable":false,"masterable":true},
        {"uniqueName":"/Lotus/Weapons/Tenno/Melee/AckAndBrunt/AckAndBrunt","name":"Ack & Brunt",
         "category":"Melee","type":"Melee","tradable":false,"masterable":true},
        {"uniqueName":"/Lotus/Weapons/Tenno/Melee/LongSword/PrismaSkana","name":"Prisma Skana",
         "category":"Melee","type":"Melee","tradable":false,"masterable":true},
        {"uniqueName":"/Lotus/Types/Items/MiscItems/IncarnonAdapters/Secondary/AtomosIncarnonUnlocker",
         "name":"Atomos Incarnon Genesis","category":"Misc","type":"Equipment Adapter","tradable":false}
    ]"#;

    fn circuit() -> Circuit {
        Circuit {
            rotates: DateTime::from_timestamp_millis(2000).unwrap(),
            normal: vec!["LokiPrime".to_owned(), "Unreleased".to_owned()],
            hard: vec![
                "AckAndBrunt".to_owned(),
                "Atomos".to_owned(),
                "Skana".to_owned(),
            ],
        }
    }

    fn rewards(rewards: &[CircuitReward]) -> Vec<(&str, Option<WorldOfferOwnership>)> {
        rewards
            .iter()
            .map(|reward| (reward.name.as_str(), reward.ownership))
            .collect()
    }

    #[test]
    fn warframes_and_incarnon_adapters() {
        let catalog = Catalog::from_json(CATALOG, "[]", "[]").unwrap();
        let account = Account::new(inventory(), &catalog, &ItemTable::build(&catalog));
        let rotation = circuit_rotation(circuit(), &catalog, Some(&account));
        assert_eq!(
            rewards(&rotation.normal),
            vec![
                ("Loki Prime", Some(WorldOfferOwnership::Mastered)),
                ("Unreleased", None),
            ]
        );
        assert_eq!(
            rewards(&rotation.hard),
            vec![
                ("Ack & Brunt", Some(WorldOfferOwnership::Missing)),
                ("Atomos", Some(WorldOfferOwnership::Owned)),
                ("Skana", Some(WorldOfferOwnership::Missing)),
            ]
        );

        let installed = inventory_with(|value| {
            entries(value, "Melee").push(json!({
                "ItemType": "/Lotus/Weapons/Tenno/Melee/LongSword/PrismaSkana",
                "ItemId": { "$oid": "00000000000000000000beef" },
                "XP": 0,
                "SkillTree": "0121",
            }));
        });
        let account = Account::new(installed, &catalog, &ItemTable::build(&catalog));
        let rotation = circuit_rotation(circuit(), &catalog, Some(&account));
        assert_eq!(rotation.hard[2].ownership, Some(WorldOfferOwnership::Owned));

        let offline = circuit_rotation(circuit(), &catalog, None);
        assert_eq!(offline.hard[1].ownership, None);
    }
}
