use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use wf_inventory::Inventory;
use wf_log::Event as LogEvent;
use wf_worldstate::WorldState;

use super::alerts::{AlertSettings, fissure_planet};
use super::{CoreEvent, FissureInfo, InventorySummary};
use crate::catalog::DUCATS_ITEM;
use crate::trade::{Trade, parse_trade_description};

fn conversation_player(channel: &str) -> Option<String> {
    let name = channel
        .strip_prefix('F')?
        .trim_end_matches(|character: char| !character.is_ascii())
        .trim();
    (!name.is_empty()).then(|| name.to_owned())
}

pub(crate) struct Engine {
    settings: AlertSettings,
    equipped_relic: Option<String>,
    pending_trade: Option<Trade>,
    reward_screen: RewardScreenState,
    world_state_polled: bool,
    announced_fissures: HashMap<String, DateTime<Utc>>,
    announced_timers: HashMap<String, DateTime<Utc>>,
    seen_channels: HashMap<String, DateTime<Utc>>,
}

#[derive(Debug, Default)]
struct RewardScreenState {
    generation: u64,
    players: HashSet<String>,
    scan_started: bool,
    announced: usize,
}

const ANNOUNCED_CAP: usize = 250;
const ANNOUNCED_MAX_AGE_SECS: i64 = 3 * 60 * 60;

fn remember(seen: &mut HashMap<String, DateTime<Utc>>, key: String, now: DateTime<Utc>) -> bool {
    let unseen = seen.insert(key, now).is_none();
    if seen.len() > ANNOUNCED_CAP {
        seen.retain(|_, at| now.signed_duration_since(*at).num_seconds() < ANNOUNCED_MAX_AGE_SECS);
    }
    unseen
}

impl Engine {
    pub fn new(settings: AlertSettings) -> Self {
        Self {
            settings,
            equipped_relic: None,
            pending_trade: None,
            reward_screen: RewardScreenState::default(),
            world_state_polled: false,
            announced_fissures: HashMap::new(),
            announced_timers: HashMap::new(),
            seen_channels: HashMap::new(),
        }
    }

    pub fn set_settings(&mut self, settings: AlertSettings) {
        self.settings = settings;
    }

    pub fn handle_log_event(&mut self, event: &LogEvent, now: DateTime<Utc>) -> Vec<CoreEvent> {
        match event {
            LogEvent::RelicEquipDialog { relic } => {
                self.equipped_relic = Some(relic.clone());
                Vec::new()
            }
            LogEvent::RelicRewardScreenOpened => {
                self.reward_screen = RewardScreenState {
                    generation: self.reward_screen.generation + 1,
                    ..RewardScreenState::default()
                };
                Vec::new()
            }
            LogEvent::OwnRelicReward { account_id, .. }
            | LogEvent::SquadRewardInfoReceived { account_id } => {
                self.reward_screen.players.insert(account_id.clone());
                Vec::new()
            }
            LogEvent::TradeDialogOpened { description } => {
                self.pending_trade = parse_trade_description(description);
                Vec::new()
            }
            LogEvent::TradeSuccessful => match self.pending_trade.take() {
                Some(trade) => vec![CoreEvent::TradeCompleted {
                    at: now,
                    partner: None,
                    trade,
                }],
                None => Vec::new(),
            },
            LogEvent::ChatTabAdded { channel } => self.new_conversation(channel, now),
            _ => Vec::new(),
        }
    }

    pub fn handle_world_state(&mut self, world: &WorldState, now: DateTime<Utc>) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        let first_poll = !self.world_state_polled;
        self.world_state_polled = true;
        for fissure in world.fissures(now) {
            let key = format!("{}@{}", fissure.node_id, fissure.expiry.timestamp());
            let unseen = remember(&mut self.announced_fissures, key, now);
            if first_poll || !unseen || !self.settings.wants(&fissure) {
                continue;
            }
            events.push(CoreEvent::FissureAlert {
                fissure: FissureInfo {
                    node_id: fissure.node_id.clone(),
                    node_name: fissure.node_name,
                    mission_type: fissure.mission_type.clone(),
                    mission_name: fissure.mission_name.clone(),
                    planet: fissure_planet(&fissure),
                    tier: fissure.tier.name().to_owned(),
                    steel_path: fissure.steel_path,
                    expiry: fissure.expiry,
                    remaining_secs: fissure.expiry.signed_duration_since(now).num_seconds(),
                },
            });
        }
        for timer in world.timers(now) {
            if !self.settings.timers.enabled(timer.name, timer.next_state) {
                continue;
            }
            let remaining_secs = timer.ends.signed_duration_since(now).num_seconds();
            if remaining_secs < 0 || remaining_secs > self.settings.timer_lead_secs {
                continue;
            }
            let key = format!("{}@{}", timer.name, timer.ends.timestamp());
            if !remember(&mut self.announced_timers, key, now) {
                continue;
            }
            events.push(CoreEvent::TimerAlert {
                name: timer.name.to_owned(),
                next_state: timer.next_state.to_owned(),
                ends_at: timer.ends,
                remaining_secs,
            });
        }
        events
    }

    pub fn reward_screen_generation(&self) -> u64 {
        self.reward_screen.generation
    }

    pub fn reward_player_count(&self) -> usize {
        self.reward_screen.players.len().max(1)
    }

    pub fn start_reward_scan(&mut self) -> bool {
        !std::mem::replace(&mut self.reward_screen.scan_started, true)
    }

    pub fn handle_reward_screen(
        &mut self,
        generation: u64,
        rewards: Vec<String>,
    ) -> Vec<CoreEvent> {
        if generation != self.reward_screen.generation
            || rewards.len() <= self.reward_screen.announced
        {
            return Vec::new();
        }
        self.reward_screen.announced = rewards.len();
        vec![CoreEvent::RelicRewardScreen {
            relic: self.equipped_relic.clone(),
            rewards,
        }]
    }

    fn new_conversation(&mut self, channel: &str, now: DateTime<Utc>) -> Vec<CoreEvent> {
        let Some(player) = conversation_player(channel) else {
            return Vec::new();
        };
        if !remember(&mut self.seen_channels, channel.to_owned(), now) {
            return Vec::new();
        }
        vec![CoreEvent::NewConversation {
            channel: channel.to_owned(),
            player,
        }]
    }
}

pub(crate) fn inventory_events(inventory: &Inventory, changes: usize) -> Vec<CoreEvent> {
    vec![CoreEvent::InventoryUpdated(InventorySummary {
        last_sync_oid: inventory.last_inventory_sync.as_str().to_owned(),
        mr: inventory.player_level,
        plat: inventory.premium_credits,
        credits: inventory.regular_credits,
        endo: inventory.fusion_points,
        ducats: inventory.counted(DUCATS_ITEM),
        changes,
    })]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::events::{CyclePhase, FissureFilter, SteelPathFilter, TimerAlerts};
    use wf_worldstate::{Fissure, FissureTier, RelicTier};

    const WORLD_STATE: &str = include_str!("../../../../fixtures/worldState.json");
    const EARLY_POLL_MILLIS: i64 = 1_788_801_669_000;
    const POLL_MILLIS: i64 = 1_788_807_069_000;

    fn at(millis: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(millis).unwrap()
    }

    fn sample() -> Fissure {
        Fissure {
            node_id: "SolNode58".to_owned(),
            node_name: Some("Galatea (Neptune)"),
            mission_type: "MT_EXTERMINATION".to_owned(),
            mission_name: "Extermination".to_owned(),
            tier: FissureTier::Relic(RelicTier::Lith),
            steel_path: false,
            is_storm: false,
            activation: at(0),
            expiry: at(1_000_000),
        }
    }

    #[test]
    fn storms_never_alert() {
        let settings = AlertSettings {
            fissure_notifications_enabled: true,
            fissure_filters: vec![FissureFilter::default()],
            ..AlertSettings::default()
        };
        assert!(settings.wants(&sample()));
        let storm = Fissure {
            is_storm: true,
            ..sample()
        };
        assert!(!settings.wants(&storm));
    }

    fn screen() -> Vec<String> {
        vec![
            "/Lotus/StoreItems/Types/Recipes/Weapons/PrimeDaikyuBlueprint".to_owned(),
            "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint".to_owned(),
        ]
    }

    fn matching_settings() -> AlertSettings {
        AlertSettings {
            fissure_notifications_enabled: true,
            fissure_filters: vec![FissureFilter::default()],
            ..AlertSettings::default()
        }
    }

    fn world() -> WorldState {
        WorldState::parse(WORLD_STATE).expect("world state")
    }

    #[test]
    fn reward_screen_once_per_generation() {
        let mut engine = Engine::new(AlertSettings::default());
        let now = at(1_000_000);
        assert!(
            engine
                .handle_log_event(
                    &LogEvent::RelicEquipDialog {
                        relic: "Lith K12".to_owned()
                    },
                    now
                )
                .is_empty()
        );
        assert!(
            engine
                .handle_log_event(&LogEvent::RelicRewardScreenOpened, now)
                .is_empty()
        );

        let generation = engine.reward_screen_generation();
        assert!(engine.start_reward_scan());
        assert!(!engine.start_reward_scan());
        assert_eq!(
            engine.handle_reward_screen(generation, screen()),
            vec![CoreEvent::RelicRewardScreen {
                relic: Some("Lith K12".to_owned()),
                rewards: screen(),
            }]
        );
        assert!(engine.handle_reward_screen(generation, screen()).is_empty());

        engine.handle_log_event(&LogEvent::RelicRewardScreenOpened, now);
        assert!(engine.start_reward_scan());
        assert!(engine.handle_reward_screen(generation, screen()).is_empty());
        let next = engine.reward_screen_generation();
        assert_eq!(engine.handle_reward_screen(next, screen()).len(), 1);
    }

    #[test]
    fn fuller_rescan() {
        let mut engine = Engine::new(AlertSettings::default());
        let generation = engine.reward_screen_generation();
        let mut partial = screen();
        partial.truncate(1);
        assert_eq!(engine.handle_reward_screen(generation, partial).len(), 1);
        assert_eq!(engine.handle_reward_screen(generation, screen()).len(), 1);
        assert!(engine.handle_reward_screen(generation, screen()).is_empty());
    }

    #[test]
    fn empty_reward_screen() {
        let mut engine = Engine::new(AlertSettings::default());
        let generation = engine.reward_screen_generation();
        assert!(
            engine
                .handle_reward_screen(generation, Vec::new())
                .is_empty()
        );
    }

    #[test]
    fn squad_player_count() {
        let mut engine = Engine::new(AlertSettings::default());
        let now = at(0);
        assert_eq!(engine.reward_player_count(), 1);
        for account in ["a", "b", "c"] {
            engine.handle_log_event(
                &LogEvent::SquadRewardInfoReceived {
                    account_id: account.to_owned(),
                },
                now,
            );
        }
        engine.handle_log_event(
            &LogEvent::OwnRelicReward {
                account_id: "a".to_owned(),
                store_item: "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint".to_owned(),
            },
            now,
        );
        assert_eq!(engine.reward_player_count(), 3);
        engine.handle_log_event(&LogEvent::RelicRewardScreenOpened, now);
        assert_eq!(engine.reward_player_count(), 1);
    }

    #[test]
    fn completed_trade() {
        let mut engine = Engine::new(AlertSettings::default());
        let now = at(2_000_000);
        engine.handle_log_event(
            &LogEvent::TradeDialogOpened {
                description: "Are you sure you want to accept this trade? You are offering 2 x Forma Blueprint in exchange for 45 Platinum.".to_owned(),
            },
            now,
        );
        let events = engine.handle_log_event(&LogEvent::TradeSuccessful, now);
        assert_eq!(events.len(), 1);
        match &events[0] {
            CoreEvent::TradeCompleted { at, partner, trade } => {
                assert_eq!(*at, now);
                assert_eq!(*partner, None);
                assert_eq!(trade.plat, 45);
                assert_eq!(trade.offered[0].count, 2);
            }
            other => panic!("unexpected event {other:?}"),
        }
        assert!(
            engine
                .handle_log_event(&LogEvent::TradeSuccessful, now)
                .is_empty()
        );
    }

    #[test]
    fn conversation_player_channels() {
        assert_eq!(
            conversation_player("Fexampletenno"),
            Some("exampletenno".to_owned())
        );
        assert_eq!(
            conversation_player("FChanie121"),
            Some("Chanie121".to_owned())
        );
        assert_eq!(
            conversation_player("Fsomeone\u{e000}"),
            Some("someone".to_owned())
        );
        assert_eq!(conversation_player("Q_EN_EU"), None);
        assert_eq!(conversation_player("R_EN_EU"), None);
        assert_eq!(conversation_player("T_EN_EU"), None);
        assert_eq!(conversation_player("S6a9f135d000000000000ab01"), None);
        assert_eq!(conversation_player("H_TRADE_EU_EN_0"), None);
        assert_eq!(conversation_player("FChanie121"), Some("Chanie121".into()));
        assert_eq!(conversation_player("F"), None);
        assert_eq!(conversation_player("F\u{e000}"), None);
    }

    #[test]
    fn repeated_chat_tab() {
        let mut engine = Engine::new(AlertSettings::default());
        let now = at(0);
        let tab = LogEvent::ChatTabAdded {
            channel: "Fexampletenno".to_owned(),
        };
        let mut emitted = Vec::new();
        for _ in 0..5 {
            emitted.extend(engine.handle_log_event(&tab, now));
        }
        assert_eq!(
            emitted,
            vec![CoreEvent::NewConversation {
                channel: "Fexampletenno".to_owned(),
                player: "exampletenno".to_owned(),
            }]
        );
    }

    #[test]
    fn public_chat_tabs() {
        let mut engine = Engine::new(AlertSettings::default());
        let now = at(0);
        for channel in [
            "Q_EN_EU",
            "R_EN_EU",
            "T_EN_EU",
            "H_TRADE_EU_EN_0",
            "S6a9f135d000000000000ab01",
        ] {
            assert!(
                engine
                    .handle_log_event(
                        &LogEvent::ChatTabAdded {
                            channel: channel.to_owned()
                        },
                        now
                    )
                    .is_empty()
            );
        }
    }

    #[test]
    fn fissure_master_switch_off() {
        let settings = AlertSettings {
            fissure_filters: vec![FissureFilter::default()],
            ..AlertSettings::default()
        };
        assert!(!settings.wants(&sample()));

        let mut engine = Engine::new(settings);
        let world = world();
        assert!(
            engine
                .handle_world_state(&world, at(EARLY_POLL_MILLIS))
                .is_empty()
        );
        assert!(
            engine
                .handle_world_state(&world, at(POLL_MILLIS))
                .is_empty()
        );
    }

    #[test]
    fn first_poll_is_silent() {
        let world = world();
        let mut engine = Engine::new(matching_settings());
        assert!(
            engine
                .handle_world_state(&world, at(POLL_MILLIS))
                .is_empty()
        );
        assert!(
            engine
                .handle_world_state(&world, at(POLL_MILLIS))
                .is_empty()
        );
    }

    #[test]
    fn new_fissures_announced_once() {
        let world = world();
        let mut engine = Engine::new(matching_settings());
        assert!(
            engine
                .handle_world_state(&world, at(EARLY_POLL_MILLIS))
                .is_empty()
        );
        let events = engine.handle_world_state(&world, at(POLL_MILLIS));
        assert!(!events.is_empty());
        for event in &events {
            match event {
                CoreEvent::FissureAlert { fissure } => {
                    assert!(fissure.remaining_secs > 0);
                    assert_ne!(fissure.mission_name, "Unknown");
                }
                other => panic!("unexpected event {other:?}"),
            }
        }
        assert!(
            engine
                .handle_world_state(&world, at(POLL_MILLIS))
                .is_empty()
        );
    }

    #[test]
    fn steel_path_lith_filter() {
        let world = world();
        let settings = AlertSettings {
            fissure_notifications_enabled: true,
            fissure_filters: vec![FissureFilter {
                tier: "Lith".to_owned(),
                steel_path: SteelPathFilter::SteelPath,
                ..FissureFilter::default()
            }],
            ..AlertSettings::default()
        };
        let mut engine = Engine::new(settings);
        engine.handle_world_state(&world, at(EARLY_POLL_MILLIS));
        let events = engine.handle_world_state(&world, at(POLL_MILLIS));
        assert!(!events.is_empty());
        for event in &events {
            match event {
                CoreEvent::FissureAlert { fissure } => {
                    assert_eq!(fissure.tier, "Lith");
                    assert!(fissure.steel_path);
                }
                other => panic!("unexpected event {other:?}"),
            }
        }
    }

    #[test]
    fn timer_inside_lead_window() {
        let world = world();
        let settings = AlertSettings {
            timers: TimerAlerts::from_iter([CyclePhase::EarthDay, CyclePhase::EarthNight]),
            timer_lead_secs: 86_400,
            ..AlertSettings::default()
        };
        let mut engine = Engine::new(settings);
        let now = at(POLL_MILLIS);
        let events = engine.handle_world_state(&world, now);
        assert_eq!(events.len(), 1);
        match &events[0] {
            CoreEvent::TimerAlert {
                name,
                next_state,
                remaining_secs,
                ..
            } => {
                assert_eq!(name, "Earth");
                assert!(next_state == "day" || next_state == "night");
                assert!(*remaining_secs >= 0);
            }
            other => panic!("unexpected event {other:?}"),
        }
        assert!(engine.handle_world_state(&world, now).is_empty());
    }

    #[test]
    fn timers_disabled() {
        let world = world();
        let settings = AlertSettings {
            timer_lead_secs: 86_400,
            ..AlertSettings::default()
        };
        let mut engine = Engine::new(settings);
        assert!(
            engine
                .handle_world_state(&world, at(POLL_MILLIS))
                .is_empty()
        );
        assert!(!TimerAlerts::default().enabled("Duviri", "joy"));
        assert!(
            !TimerAlerts::from_iter([
                CyclePhase::EarthDay,
                CyclePhase::CetusNight,
                CyclePhase::VallisCold,
                CyclePhase::CambionFass,
            ])
            .enabled("Zariman", "corpus")
        );
    }

    #[test]
    fn arriving_phase_only() {
        let world = world();
        let now = at(POLL_MILLIS);
        let coming = world
            .timers(now)
            .into_iter()
            .find(|timer| timer.name == "Orb Vallis")
            .unwrap();
        let leaving = AlertSettings {
            timers: CyclePhase::of("Orb Vallis", coming.state)
                .into_iter()
                .collect(),
            timer_lead_secs: 86_400,
            ..AlertSettings::default()
        };
        assert!(
            Engine::new(leaving)
                .handle_world_state(&world, now)
                .is_empty()
        );

        let arriving = AlertSettings {
            timers: CyclePhase::of("Orb Vallis", coming.next_state)
                .into_iter()
                .collect(),
            timer_lead_secs: 86_400,
            ..AlertSettings::default()
        };
        let events = Engine::new(arriving).handle_world_state(&world, now);
        assert_eq!(events.len(), 1);
        match &events[0] {
            CoreEvent::TimerAlert {
                name, next_state, ..
            } => {
                assert_eq!(name, "Orb Vallis");
                assert_eq!(next_state, coming.next_state);
            }
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn announced_set_cap() {
        let mut seen = HashMap::new();
        let start = at(POLL_MILLIS);
        for index in 0..ANNOUNCED_CAP {
            assert!(remember(&mut seen, format!("stale-{index}"), start));
        }
        let later = start + chrono::TimeDelta::seconds(ANNOUNCED_MAX_AGE_SECS + 1);
        assert!(remember(&mut seen, "fresh".to_owned(), later));
        assert_eq!(seen.len(), 1);
        assert!(seen.contains_key("fresh"));
    }

    #[test]
    fn inventory_summary() {
        let inventory = fixtures::inventory();
        let events = inventory_events(&inventory, 7);
        assert_eq!(
            events,
            vec![CoreEvent::InventoryUpdated(InventorySummary {
                last_sync_oid: "6a9eeb1f000000000000c001".to_owned(),
                mr: 14,
                plat: 5249,
                credits: 90_814_797,
                endo: 189_082,
                ducats: 937,
                changes: 7,
            })]
        );
    }
}
