use serde::Deserialize;

use crate::mongo::{MongoDate, ObjectId};
use crate::riven::RivenFingerprint;
use crate::{InventoryError, Result};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArchonCrystalSlot {
    #[serde(rename_all = "PascalCase")]
    Installed {
        color: String,
        upgrade_type: String,
    },
    Empty([u8; 0]),
}

impl ArchonCrystalSlot {
    pub fn is_installed(&self) -> bool {
        matches!(self, Self::Installed { .. })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LoadoutConfig {
    #[serde(default)]
    pub upgrades: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EquipmentItem {
    pub item_type: String,
    pub item_id: ObjectId,
    #[serde(default, rename = "XP")]
    pub xp: u64,
    pub features: Option<u32>,
    pub polarized: Option<u32>,
    pub upgrade_ver: Option<u32>,
    pub skill_tree: Option<String>,
    #[serde(default)]
    pub configs: Vec<LoadoutConfig>,
    #[serde(default)]
    pub archon_crystal_upgrades: Vec<ArchonCrystalSlot>,
}

impl EquipmentItem {
    pub fn archon_shards(&self) -> u32 {
        self.archon_crystal_upgrades
            .iter()
            .map(|slot| u32::from(slot.is_installed()))
            .sum()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CountedItem {
    pub item_type: String,
    pub item_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CosmeticItem {
    pub item_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PetTraits {
    pub personality: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PetPrint {
    pub dominant_traits: PetTraits,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Upgrade {
    pub item_type: String,
    pub item_id: ObjectId,
    pub upgrade_fingerprint: Option<String>,
}

impl Upgrade {
    pub fn fingerprint(&self) -> Option<RivenFingerprint> {
        RivenFingerprint::parse(self.upgrade_fingerprint.as_deref()?).ok()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PendingRecipe {
    pub item_type: String,
    pub completion_date: MongoDate,
    pub item_id: ObjectId,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XpInfo {
    pub item_type: String,
    #[serde(rename = "XP")]
    pub xp: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mission {
    pub tag: String,
    pub completes: u32,
    pub tier: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlayerSkills {
    #[serde(default, rename = "LPS_TACTICAL")]
    pub tactical: u32,
    #[serde(default, rename = "LPS_PILOTING")]
    pub piloting: u32,
    #[serde(default, rename = "LPS_GUNNERY")]
    pub gunnery: u32,
    #[serde(default, rename = "LPS_ENGINEERING")]
    pub engineering: u32,
    #[serde(default, rename = "LPS_COMMAND")]
    pub command: u32,
    #[serde(default, rename = "LPS_DRIFT_COMBAT")]
    pub drift_combat: u32,
    #[serde(default, rename = "LPS_DRIFT_RIDING")]
    pub drift_riding: u32,
    #[serde(default, rename = "LPS_DRIFT_OPPORTUNITY")]
    pub drift_opportunity: u32,
    #[serde(default, rename = "LPS_DRIFT_ENDURANCE")]
    pub drift_endurance: u32,
}

impl PlayerSkills {
    pub fn railjack(&self) -> u32 {
        self.tactical + self.piloting + self.gunnery + self.engineering + self.command
    }

    pub fn duviri(&self) -> u32 {
        self.drift_combat + self.drift_riding + self.drift_opportunity + self.drift_endurance
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Affiliation {
    pub tag: String,
    #[serde(default)]
    pub standing: i64,
    #[serde(default)]
    pub title: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChallengeProgress {
    pub name: String,
    pub progress: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FocusUpgrade {
    pub item_type: String,
    #[serde(default)]
    pub level: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Booster {
    pub item_type: String,
    pub expiry_date: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConsumedSuit {
    #[serde(rename = "s")]
    pub suit_type: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InfestedFoundry {
    #[serde(default)]
    pub consumed_suits: Vec<ConsumedSuit>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Accolades {
    pub founder: Option<serde_json::Value>,
}

impl Accolades {
    pub fn is_founder(&self) -> bool {
        match &self.founder {
            None | Some(serde_json::Value::Null) => false,
            Some(serde_json::Value::Bool(flag)) => *flag,
            Some(serde_json::Value::Number(level)) => level.as_f64().is_some_and(|v| v > 0.0),
            Some(serde_json::Value::String(text)) => !text.is_empty(),
            Some(_) => true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Inventory {
    pub player_level: u32,
    pub premium_credits: i64,
    pub premium_credits_free: i64,
    pub regular_credits: i64,
    pub fusion_points: i64,
    pub trades_remaining: u32,
    pub created: MongoDate,
    pub last_inventory_sync: ObjectId,
    pub reward_seed: i64,
    #[serde(default)]
    pub suits: Vec<EquipmentItem>,
    #[serde(default)]
    pub long_guns: Vec<EquipmentItem>,
    #[serde(default)]
    pub pistols: Vec<EquipmentItem>,
    #[serde(default)]
    pub melee: Vec<EquipmentItem>,
    #[serde(default)]
    pub space_suits: Vec<EquipmentItem>,
    #[serde(default)]
    pub space_guns: Vec<EquipmentItem>,
    #[serde(default)]
    pub space_melee: Vec<EquipmentItem>,
    #[serde(default)]
    pub sentinels: Vec<EquipmentItem>,
    #[serde(default)]
    pub sentinel_weapons: Vec<EquipmentItem>,
    #[serde(default)]
    pub mech_suits: Vec<EquipmentItem>,
    #[serde(default)]
    pub hoverboards: Vec<EquipmentItem>,
    #[serde(default)]
    pub moa_pets: Vec<EquipmentItem>,
    #[serde(default)]
    pub kubrow_pets: Vec<EquipmentItem>,
    #[serde(default)]
    pub data_knives: Vec<EquipmentItem>,
    #[serde(default)]
    pub crew_ship_harnesses: Vec<EquipmentItem>,
    #[serde(default)]
    pub misc_items: Vec<CountedItem>,
    #[serde(default)]
    pub recipes: Vec<CountedItem>,
    #[serde(default)]
    pub raw_upgrades: Vec<CountedItem>,
    #[serde(default)]
    pub consumables: Vec<CountedItem>,
    #[serde(default)]
    pub level_keys: Vec<CountedItem>,
    #[serde(default)]
    pub fusion_treasures: Vec<CountedItem>,
    #[serde(default)]
    pub weapon_skins: Vec<CosmeticItem>,
    #[serde(default)]
    pub flavour_items: Vec<CosmeticItem>,
    #[serde(default)]
    pub kubrow_pet_prints: Vec<PetPrint>,
    #[serde(default)]
    pub upgrades: Vec<Upgrade>,
    #[serde(default)]
    pub pending_recipes: Vec<PendingRecipe>,
    #[serde(default, rename = "XPInfo")]
    pub xp_info: Vec<XpInfo>,
    #[serde(default)]
    pub missions: Vec<Mission>,
    #[serde(default)]
    pub player_skills: PlayerSkills,
    #[serde(default)]
    pub affiliations: Vec<Affiliation>,
    #[serde(default)]
    pub challenge_progress: Vec<ChallengeProgress>,
    #[serde(default)]
    pub load_out_presets: serde_json::Value,
    #[serde(default)]
    pub focus_upgrades: Vec<FocusUpgrade>,
    #[serde(default)]
    pub boosters: Vec<Booster>,
    #[serde(default)]
    pub accolades: Accolades,
    #[serde(default)]
    pub infested_foundry: InfestedFoundry,
}

impl Inventory {
    pub fn parse(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(InventoryError::from)
    }
}
