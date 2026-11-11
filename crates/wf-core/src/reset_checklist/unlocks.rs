use std::collections::HashSet;

use wf_inventory::Inventory;

use crate::reset_checklist::syndicate_missions::syndicate_missions_open;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Quest {
    WarWithin,
    SecondDream,
    NewStrange,
    Veilbreaker,
    WhispersInTheWalls,
    TheHex,
    DuviriParadox,
    OldPeace,
    ManOfFewWords,
}

impl Quest {
    fn key_chain(self) -> &'static str {
        match self {
            Self::WarWithin => "/Lotus/Types/Keys/WarWithinQuest/WarWithinQuestKeyChain",
            Self::SecondDream => "/Lotus/Types/Keys/OrokinMoonQuest/OrokinMoonQuestKeyChain",
            Self::NewStrange => "/Lotus/Types/Keys/DragonQuest/DragonQuestKeyChain",
            Self::Veilbreaker => "/Lotus/Types/Keys/KahlQuest/KahlQuestKeyChain",
            Self::WhispersInTheWalls => "/Lotus/Types/Keys/EntratiLab/EntratiQuestKeyChain",
            Self::TheHex => "/Lotus/Types/Keys/1999Quest/1999QuestKeyChain",
            Self::DuviriParadox => "/Lotus/Types/Keys/DuviriQuest/DuviriQuestKeyChain",
            Self::OldPeace => "/Lotus/Types/Keys/TauPrequel/TauPrequelQuestKeyChain",
            Self::ManOfFewWords => "/Lotus/Types/Keys/GetClemQuest/GetClemQuestKeyChain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SyndicateRank {
    tag: &'static str,
    rank: i32,
}

impl SyndicateRank {
    pub(crate) const CAVIA_5: Self = Self {
        tag: "EntratiLabSyndicate",
        rank: 5,
    };
    pub(crate) const HEX_5: Self = Self {
        tag: "HexSyndicate",
        rank: 5,
    };

    fn reached(self, inventory: &Inventory) -> bool {
        inventory
            .affiliations
            .iter()
            .any(|affiliation| affiliation.tag == self.tag && affiliation.title >= self.rank)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResetChecklistRequirement {
    Quest(Quest),
    SyndicateRank(SyndicateRank),
    AnyFactionSyndicateRanked,
    DialogueFlag(&'static str),
    EliteArchimedeaUnlocked,
}

impl ResetChecklistRequirement {
    fn met(self, inventory: &Inventory, completed_quests: &HashSet<&str>) -> bool {
        match self {
            Self::Quest(quest) => completed_quests.contains(quest.key_chain()),
            Self::SyndicateRank(rank) => rank.reached(inventory),
            Self::AnyFactionSyndicateRanked => syndicate_missions_open(inventory),
            Self::DialogueFlag(flag) => inventory.dialogue_flag_set(flag),
            Self::EliteArchimedeaUnlocked => inventory.elite_archimedea_unlocked(),
        }
    }
}

pub(crate) fn locked(
    requirements: &[ResetChecklistRequirement],
    inventory: &Inventory,
    completed_quests: &HashSet<&str>,
) -> bool {
    requirements
        .iter()
        .any(|requirement| !requirement.met(inventory, completed_quests))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::catalog::fixtures;

    fn locked_for(inventory: &Inventory, requirements: &[ResetChecklistRequirement]) -> bool {
        locked(requirements, inventory, &inventory.completed_quests())
    }

    #[test]
    fn quest_and_rank() {
        let inventory = fixtures::inventory();

        assert!(!locked_for(&inventory, &[]));
        assert!(!locked_for(
            &inventory,
            &[ResetChecklistRequirement::Quest(Quest::WhispersInTheWalls)]
        ));
        assert!(locked_for(
            &inventory,
            &[ResetChecklistRequirement::Quest(Quest::WarWithin)]
        ));
        assert!(!locked_for(
            &inventory,
            &[ResetChecklistRequirement::SyndicateRank(
                SyndicateRank::CAVIA_5
            )]
        ));
        assert!(locked_for(
            &inventory,
            &[
                ResetChecklistRequirement::Quest(Quest::TheHex),
                ResetChecklistRequirement::SyndicateRank(SyndicateRank::HEX_5)
            ]
        ));
    }

    #[test]
    fn rank_below() {
        let inventory = fixtures::inventory_with(|value| {
            for affiliation in fixtures::entries(value, "Affiliations") {
                if affiliation["Tag"] == "EntratiLabSyndicate" {
                    affiliation["Title"] = 4.into();
                }
            }
        });

        assert!(locked_for(
            &inventory,
            &[ResetChecklistRequirement::SyndicateRank(
                SyndicateRank::CAVIA_5
            )]
        ));
    }

    #[test]
    fn faction_syndicate_ranked() {
        let unranked = fixtures::inventory_with(|value| {
            for affiliation in fixtures::entries(value, "Affiliations") {
                affiliation["Title"] = 0.into();
            }
        });

        let requirement = [ResetChecklistRequirement::AnyFactionSyndicateRanked];

        assert!(!locked_for(&fixtures::inventory(), &requirement));
        assert!(locked_for(&unranked, &requirement));
    }

    #[test]
    fn elite_archimedea() {
        let elite_temporal = fixtures::inventory_with(|value| {
            value["EchoesHexConquestHardModeStatus"] = json!(1);
        });

        let requirement = [ResetChecklistRequirement::EliteArchimedeaUnlocked];

        assert!(locked_for(&fixtures::inventory(), &requirement));
        assert!(!locked_for(&elite_temporal, &requirement));
    }
}
