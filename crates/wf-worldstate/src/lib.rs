mod archon;
mod circuit;
mod cycles;
mod daily_deal;
mod embedded;
mod error;
#[cfg(feature = "fetch")]
mod fetch;
mod fissure;
mod flash_sale;
mod manifest;
mod mission_types;
mod mongo_date;
mod nodes;
mod prime;
mod season;
mod sortie;
mod syndicate;
mod void_trader;

pub use archon::{ArchonHunt, ArchonMission};
pub use circuit::Circuit;
pub use cycles::Timer;
pub use daily_deal::DailyDeal;
pub use error::{Result, WorldStateError};
#[cfg(feature = "fetch")]
pub use fetch::{fetch, fetch_body};

pub const WORLD_STATE_URL: &str = "https://api.warframe.com/cdn/worldState.php";
pub use fissure::{Fissure, FissureTier, RelicTier};
pub use flash_sale::MarketSale;
pub use manifest::ManifestItem;
pub use mission_types::mission_type_name;
pub use nodes::{
    SolNode, is_masterable_node, junction_count, junction_mastery_xp, junction_nodes,
    masterable_node_count, masterable_node_xp_total, masterable_nodes, node_info, node_mastery_xp,
    node_name,
};
pub use prime::{PrimeAccessAvailability, Varzia, VarziaItem};
pub use season::{ChallengeKind, NightwaveChallenge, NightwaveSeason};
pub use sortie::{Sortie, SortieMission};
pub use void_trader::BaroStatus;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use archon::LiteSortie;
use circuit::EndlessXpWeek;
use daily_deal::RawDailyDeal;
use fissure::{ActiveMission, VoidStorm};
use flash_sale::RawFlashSale;
use prime::{PrimeVaultTrader, RawPrimeAccessAvailability};
use season::SeasonInfo;
use sortie::RawSortie;
use syndicate::SyndicateMission;
use void_trader::VoidTrader;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawWorldState {
    #[serde(default)]
    active_missions: Vec<ActiveMission>,
    #[serde(default)]
    void_storms: Vec<VoidStorm>,
    #[serde(default)]
    void_traders: Vec<VoidTrader>,
    #[serde(default)]
    sorties: Vec<RawSortie>,
    #[serde(default)]
    lite_sorties: Vec<LiteSortie>,
    #[serde(default)]
    daily_deals: Vec<RawDailyDeal>,
    #[serde(default)]
    flash_sales: Vec<RawFlashSale>,
    #[serde(default)]
    season_info: Option<SeasonInfo>,
    #[serde(default)]
    prime_vault_traders: Vec<PrimeVaultTrader>,
    #[serde(default)]
    prime_access_availability: Option<RawPrimeAccessAvailability>,
    #[serde(default)]
    syndicate_missions: Vec<SyndicateMission>,
    #[serde(default)]
    endless_xp_schedule: Vec<EndlessXpWeek>,
}

#[derive(Debug, Clone)]
pub struct WorldState {
    raw: RawWorldState,
}

impl WorldState {
    pub fn parse(json: &str) -> Result<Self> {
        let raw: RawWorldState = serde_json::from_str(json)?;
        Ok(Self { raw })
    }

    pub fn fissures(&self, now: DateTime<Utc>) -> Vec<Fissure> {
        fissure::active_fissures(&self.raw.active_missions, &self.raw.void_storms, now)
    }

    pub fn baro(&self, now: DateTime<Utc>) -> Option<BaroStatus> {
        void_trader::baro_status(&self.raw.void_traders, now)
    }

    pub fn timers(&self, now: DateTime<Utc>) -> Vec<Timer> {
        cycles::timers(&self.raw.syndicate_missions, now)
    }

    pub fn sortie(&self, now: DateTime<Utc>) -> Option<Sortie> {
        sortie::current_sortie(&self.raw.sorties, now)
    }

    pub fn archon_hunt(&self, now: DateTime<Utc>) -> Option<ArchonHunt> {
        archon::current_archon_hunt(&self.raw.lite_sorties, now)
    }

    pub fn daily_deals(&self, now: DateTime<Utc>) -> Vec<DailyDeal> {
        daily_deal::daily_deals(&self.raw.daily_deals, now)
    }

    pub fn market_sales(&self, now: DateTime<Utc>) -> Vec<MarketSale> {
        flash_sale::market_sales(&self.raw.flash_sales, now)
    }

    pub fn season(&self, now: DateTime<Utc>) -> Option<NightwaveSeason> {
        self.raw
            .season_info
            .as_ref()
            .map(|raw| season::current_season(raw, now))
    }

    pub fn varzia(&self, now: DateTime<Utc>) -> Option<Varzia> {
        prime::current_varzia(&self.raw.prime_vault_traders, now)
    }

    pub fn circuit(&self, now: DateTime<Utc>) -> Option<Circuit> {
        circuit::current_circuit(&self.raw.endless_xp_schedule, now)
    }

    pub fn prime_access_availability(&self) -> Option<PrimeAccessAvailability> {
        self.raw
            .prime_access_availability
            .as_ref()
            .map(|raw| PrimeAccessAvailability {
                state: raw.state.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    const FIXTURE: &str = include_str!("../../../fixtures/worldState.json");
    const FIXTURE_NOW_MS: i64 = 1_788_807_069_000;

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp_millis(FIXTURE_NOW_MS).unwrap()
    }

    #[test]
    fn fissure_node_names() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let fissures = state.fissures(now());
        assert!(!fissures.is_empty());
        assert!(fissures.iter().all(|fissure| fissure.node_name.is_some()));
    }

    #[test]
    fn fissure_mission_names() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let fissures = state.fissures(now());
        assert!(!fissures.is_empty());
        for fissure in &fissures {
            assert_ne!(fissure.mission_name, "Unknown");
            assert!(!fissure.mission_name.starts_with("MT_"));
        }
    }

    #[test]
    fn void_storms_among_fissures() {
        let state = WorldState::parse(FIXTURE).unwrap();
        assert_eq!(state.raw.void_storms.len(), 12);
        let fissures = state.fissures(now());
        let storms: Vec<_> = fissures.iter().filter(|fissure| fissure.is_storm).collect();
        assert_eq!(fissures.len(), 28);
        assert_eq!(storms.len(), 6);
        assert!(
            storms
                .iter()
                .all(|storm| storm.node_id.starts_with("CrewBattleNode"))
        );
        assert!(storms.iter().all(|storm| storm.node_name.is_some()));
        assert!(storms.iter().all(|storm| !storm.steel_path));
        assert!(storms.iter().any(|storm| storm.mission_name == "Skirmish"));
        assert!(storms.iter().any(|storm| storm.mission_name == "Volatile"));
        assert!(
            storms
                .iter()
                .any(|storm| storm.tier == FissureTier::Relic(RelicTier::Axi))
        );
    }

    #[test]
    fn fissures_sorted_by_tier() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let fissures = state.fissures(now());
        let tiers: Vec<_> = fissures.iter().map(|fissure| &fissure.tier).collect();
        let mut sorted = tiers.clone();
        sorted.sort();
        assert_eq!(tiers, sorted);
    }

    #[test]
    fn baro_status() {
        let state = WorldState::parse(FIXTURE).unwrap();
        assert!(state.baro(now()).is_some());
    }

    #[test]
    fn sortie_missions() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let sortie = state.sortie(now()).unwrap();
        assert_eq!(sortie.missions.len(), 3);
        assert_eq!(sortie.boss_name, "Kela De Thaym");
        assert_eq!(sortie.faction, Some("Grineer"));
        assert!(
            sortie
                .missions
                .iter()
                .all(|mission| !mission.modifier_name.starts_with("SORTIE_"))
        );
    }

    #[test]
    fn archon_hunt_missions() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let hunt = state.archon_hunt(now()).unwrap();
        assert_eq!(hunt.missions.len(), 3);
        assert_eq!(hunt.boss_name, "Archon Amar");
        assert_eq!(hunt.faction, Some("Narmer"));
    }

    #[test]
    fn cetus_timer() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let timers = state.timers(now());
        let cetus = timers.iter().find(|timer| timer.name == "Cetus").unwrap();
        assert!(cetus.state == "day" || cetus.state == "night");
    }

    #[test]
    fn running_daily_deal() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let deals = state.daily_deals(now());
        assert_eq!(deals.len(), 1);
        assert_eq!(
            deals[0].store_item,
            "/Lotus/StoreItems/Types/Items/Research/BioComponent"
        );
        assert_eq!(deals[0].discount_percent, 20);
        assert_eq!(deals[0].original_price, 10);
        assert_eq!(deals[0].sale_price, 8);
        assert_eq!(deals[0].amount_total, 165);
        assert_eq!(deals[0].expiry.timestamp_millis(), 1_788_814_800_000);
    }

    #[test]
    fn expired_daily_deal() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let long_after = DateTime::<Utc>::from_timestamp_millis(1_800_000_000_000).unwrap();
        assert!(state.daily_deals(long_after).is_empty());
    }

    #[test]
    fn nightwave_season() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let season = state.season(now()).unwrap();
        assert_eq!(season.season, 18);
        assert_eq!(season.phase, 0);
        assert_eq!(season.name, "Intermission 16");
        assert_eq!(season.expiry.timestamp_millis(), 1_804_464_000_000);
        assert_eq!(season.challenges.len(), 10);
        let dailies = season
            .challenges
            .iter()
            .filter(|challenge| challenge.kind == ChallengeKind::Daily)
            .count();
        assert_eq!(dailies, 3);
        let elite = season
            .challenges
            .iter()
            .filter(|challenge| challenge.kind == ChallengeKind::EliteWeekly)
            .count();
        assert_eq!(elite, 2);
        let poison = season
            .challenges
            .iter()
            .find(|challenge| challenge.tag.ends_with("SeasonDailyKillEnemiesWithPoison"))
            .unwrap();
        assert_eq!(poison.name, "Poisoner");
        assert_eq!(
            poison.description,
            Some("Kill 150 Enemies with Toxin Damage")
        );
        let titled = season
            .challenges
            .iter()
            .filter(|challenge| challenge.description.is_some())
            .count();
        assert_eq!(titled, 9);
        for challenge in &season.challenges {
            assert!(!challenge.name.contains('/'));
            assert!(!challenge.name.starts_with("Season"));
        }
    }

    #[test]
    fn prime_access_parses() {
        let state = WorldState::parse(FIXTURE).unwrap();
        assert_eq!(state.prime_access_availability().unwrap().state, "PRIME1");
    }

    #[test]
    fn varzia_rotation() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let varzia = state.varzia(now()).unwrap();
        assert_eq!(varzia.node_name, Some("Maroo's Bazaar (Mars)"));
        assert_eq!(varzia.expiry.timestamp_millis(), 1_790_877_600_000);
        assert_eq!(varzia.items.len(), 15);
        assert!(varzia.items.iter().any(|item| {
            item.item_type == "/Lotus/StoreItems/Powersuits/Banshee/BansheePrime"
                && item.regal_aya == 3
        }));
    }

    #[test]
    fn circuit_week() {
        let state = WorldState::parse(FIXTURE).unwrap();
        let circuit = state.circuit(now()).unwrap();
        assert_eq!(circuit.rotates.timestamp_millis(), 1_789_344_000_000);
        assert_eq!(circuit.normal.len(), 3);
        assert_eq!(circuit.hard.len(), 5);
        assert!(circuit.normal.contains(&"Garuda".to_owned()));
        assert!(circuit.hard.contains(&"Gammacor".to_owned()));
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(WorldState::parse("not json").is_err());
    }
}
