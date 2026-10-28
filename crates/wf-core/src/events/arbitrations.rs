use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use wf_worldstate::{
    ArbitrationForecast, ArbitrationForecastHour, ArbitrationNode, ArbitrationTier,
    ArbitrationTiers, arbitration_node,
};

pub const ARBITRATION_FORECAST_TRUSTED: TimeDelta = TimeDelta::hours(24);
const SCHEDULE_HOURS: usize = 169;
const START_WINDOW_MINUTES: i64 = 5;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct ArbitrationAlertFilter {
    pub mission_types: BTreeSet<String>,
    pub planets: BTreeSet<String>,
    pub node_ids: BTreeSet<String>,
    pub factions: BTreeSet<String>,
    pub tiers: BTreeSet<ArbitrationTier>,
    pub favourites_only: bool,
}

fn any_or_chosen(chosen: &BTreeSet<String>, value: Option<&str>) -> bool {
    chosen.is_empty() || value.is_some_and(|value| chosen.contains(value))
}

impl ArbitrationAlertFilter {
    pub fn is_empty(&self) -> bool {
        self.mission_types.is_empty()
            && self.planets.is_empty()
            && self.node_ids.is_empty()
            && self.factions.is_empty()
            && self.tiers.is_empty()
            && !self.favourites_only
    }

    fn matches(&self, node: &ArbitrationNode, favourites: &BTreeSet<String>) -> bool {
        (!self.favourites_only || favourites.contains(&node.node_id))
            && any_or_chosen(&self.node_ids, Some(&node.node_id))
            && any_or_chosen(&self.mission_types, node.mission_type)
            && any_or_chosen(&self.planets, node.planet)
            && any_or_chosen(&self.factions, node.faction)
            && (self.tiers.is_empty() || self.tiers.contains(&node.tier))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct ArbitrationAlertSettings {
    pub enabled: bool,
    pub notify_on_start: bool,
    pub lead_minutes: u32,
    pub show_on_world_page: bool,
    pub favourite_nodes: BTreeSet<String>,
    pub reminders: BTreeSet<String>,
    pub filter: ArbitrationAlertFilter,
}

impl Default for ArbitrationAlertSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            notify_on_start: true,
            lead_minutes: 10,
            show_on_world_page: true,
            favourite_nodes: BTreeSet::new(),
            reminders: BTreeSet::new(),
            filter: ArbitrationAlertFilter::default(),
        }
    }
}

impl ArbitrationAlertSettings {
    pub fn has_match_rules(&self) -> bool {
        self.enabled && !self.filter.is_empty()
    }

    fn notifies_at_start(&self) -> bool {
        self.lead_minutes == 0 || self.notify_on_start
    }

    fn lead(&self) -> TimeDelta {
        TimeDelta::minutes(i64::from(self.lead_minutes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArbitrationSlot {
    pub id: String,
    #[serde(flatten)]
    pub node: ArbitrationNode,
    pub starts: DateTime<Utc>,
    pub ends: DateTime<Utc>,
    pub favourite: bool,
    pub matches: bool,
    pub reminder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ArbitrationSchedule {
    pub slots: Vec<ArbitrationSlot>,
    pub nodes: Vec<ArbitrationNode>,
    pub has_match_rules: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArbitrationAlert {
    pub slot: ArbitrationSlot,
    pub at_start: bool,
    pub minutes_ahead: i64,
    pub reminder: bool,
}

impl ArbitrationAlert {
    pub fn key(&self) -> String {
        let moment = if self.at_start { "start" } else { "advance" };
        format!("{}:{moment}", self.slot.id)
    }
}

fn minutes_rounded_up(until: TimeDelta) -> i64 {
    (until.num_milliseconds() + 59_999) / 60_000
}

pub(crate) struct ArbitrationOutlook<'a> {
    pub forecast: &'a ArbitrationForecast,
    pub tiers: &'a ArbitrationTiers,
    pub settings: &'a ArbitrationAlertSettings,
}

impl ArbitrationOutlook<'_> {
    fn slot(&self, hour: ArbitrationForecastHour<'_>) -> ArbitrationSlot {
        let node = arbitration_node(hour.node_id, self.tiers);
        let id = format!("{}:{}", hour.starts.timestamp(), hour.node_id);
        ArbitrationSlot {
            favourite: self.settings.favourite_nodes.contains(&node.node_id),
            matches: self
                .settings
                .filter
                .matches(&node, &self.settings.favourite_nodes),
            reminder: self.settings.reminders.contains(&id),
            id,
            node,
            starts: hour.starts,
            ends: hour.ends,
        }
    }

    pub fn schedule(&self, now: DateTime<Utc>) -> Option<ArbitrationSchedule> {
        let index = self.forecast.index_at(now)?;
        let slots = self
            .forecast
            .hours_from(index, SCHEDULE_HOURS)
            .map(|hour| self.slot(hour))
            .collect();
        let mut nodes: Vec<ArbitrationNode> = self
            .forecast
            .unique_node_ids()
            .into_iter()
            .map(|node_id| arbitration_node(node_id, self.tiers))
            .collect();
        nodes.sort_by(|a, b| a.planet.cmp(&b.planet).then_with(|| a.node.cmp(&b.node)));
        Some(ArbitrationSchedule {
            slots,
            nodes,
            has_match_rules: self.settings.has_match_rules(),
        })
    }

    pub fn due_alerts(&self, now: DateTime<Utc>) -> Vec<ArbitrationAlert> {
        let Some(index) = self.forecast.index_at(now) else {
            return Vec::new();
        };
        self.forecast
            .hours_from(index, 2)
            .map(|hour| self.slot(hour))
            .filter_map(|slot| self.due_alert(slot, now))
            .collect()
    }

    fn due_alert(&self, slot: ArbitrationSlot, now: DateTime<Utc>) -> Option<ArbitrationAlert> {
        let settings = self.settings;
        let starting =
            now >= slot.starts && now < slot.starts + TimeDelta::minutes(START_WINDOW_MINUTES);
        let ahead =
            settings.lead_minutes > 0 && now < slot.starts && now >= slot.starts - settings.lead();
        let wanted = settings.enabled
            && slot.matches
            && ((starting && settings.notifies_at_start()) || ahead);
        if !(wanted || (slot.reminder && (starting || ahead))) {
            return None;
        }

        let minutes_ahead = if starting {
            0
        } else {
            minutes_rounded_up(slot.starts.signed_duration_since(now))
        };
        Some(ArbitrationAlert {
            reminder: slot.reminder,
            slot,
            at_start: starting,
            minutes_ahead,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures::{self, at};
    use crate::events::{AlertSettings, CoreEvent, Engine};

    const FIRST_HOUR_MS: i64 = 1_727_884_800_000;
    const HOUR_MS: i64 = 3_600_000;
    const MINUTE_MS: i64 = 60_000;

    fn forecast() -> ArbitrationForecast {
        ArbitrationForecast::parse(fixtures::ARBITRATIONS).unwrap()
    }

    fn tiers() -> ArbitrationTiers {
        ArbitrationTiers::parse(fixtures::ARBITRATION_TIERS).unwrap()
    }

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn schedule_at(
        settings: &ArbitrationAlertSettings,
        now_ms: i64,
    ) -> Option<ArbitrationSchedule> {
        let forecast = forecast();
        let tiers = tiers();
        ArbitrationOutlook {
            forecast: &forecast,
            tiers: &tiers,
            settings,
        }
        .schedule(at(now_ms))
    }

    fn alerts_at(
        settings: &ArbitrationAlertSettings,
        now_ms: i64,
    ) -> Vec<(String, bool, i64, bool)> {
        let forecast = forecast();
        let tiers = tiers();
        ArbitrationOutlook {
            forecast: &forecast,
            tiers: &tiers,
            settings,
        }
        .due_alerts(at(now_ms))
        .into_iter()
        .map(|alert| {
            (
                alert.key(),
                alert.at_start,
                alert.minutes_ahead,
                alert.reminder,
            )
        })
        .collect()
    }

    #[test]
    fn schedule_runs_from_the_current_hour() {
        let settings = ArbitrationAlertSettings::default();
        let schedule = schedule_at(&settings, FIRST_HOUR_MS + 2 * HOUR_MS + 1).unwrap();
        assert_eq!(schedule.slots.len(), 46);
        assert_eq!(schedule.slots[0].id, "1727892000:SolNode64");
        assert_eq!(schedule.slots[0].node.node, "Umbriel");
        assert_eq!(schedule.slots[0].node.planet, Some("Uranus"));
        assert_eq!(schedule.slots[0].node.tier, ArbitrationTier::S);
        assert_eq!(schedule.slots[0].starts, at(FIRST_HOUR_MS + 2 * HOUR_MS));
        assert_eq!(schedule.slots[0].ends, at(FIRST_HOUR_MS + 3 * HOUR_MS));
        assert!(schedule.slots.iter().all(|slot| slot.matches));
        assert!(
            schedule
                .slots
                .iter()
                .all(|slot| !slot.favourite && !slot.reminder)
        );
        assert!(!schedule.has_match_rules);
        assert_eq!(schedule.nodes.len(), 38);
        assert!(
            schedule
                .nodes
                .is_sorted_by_key(|node| (node.planet, node.node.clone()))
        );
        assert_eq!(schedule_at(&settings, FIRST_HOUR_MS - 1), None);
    }

    #[test]
    fn slots_carry_the_personal_marks() {
        let settings = ArbitrationAlertSettings {
            enabled: true,
            favourite_nodes: set(&["SolNode64"]),
            reminders: set(&["1727892000:SolNode64"]),
            filter: ArbitrationAlertFilter {
                tiers: [ArbitrationTier::S].into_iter().collect(),
                ..ArbitrationAlertFilter::default()
            },
            ..ArbitrationAlertSettings::default()
        };
        let schedule = schedule_at(&settings, FIRST_HOUR_MS).unwrap();
        assert!(schedule.has_match_rules);
        let switched_off = ArbitrationAlertSettings {
            enabled: false,
            ..settings.clone()
        };
        assert!(
            !schedule_at(&switched_off, FIRST_HOUR_MS)
                .unwrap()
                .has_match_rules
        );
        let umbriel = &schedule.slots[2];
        assert!(umbriel.favourite && umbriel.reminder && umbriel.matches);
        let first = &schedule.slots[0];
        assert_eq!(first.node.node_id, "SettlementNode11");
        assert!(!first.favourite && !first.reminder && !first.matches);
        assert_eq!(
            schedule.slots.iter().filter(|slot| slot.reminder).count(),
            1
        );
    }

    #[test]
    fn filter_groups_are_and_within_or() {
        let umbriel = arbitration_node("SolNode64", &tiers());
        let favourites = set(&["SolNode64"]);
        let none = BTreeSet::new();
        let filter = ArbitrationAlertFilter::default();
        assert!(filter.is_empty());
        assert!(filter.matches(&umbriel, &none));

        let planet = ArbitrationAlertFilter {
            planets: set(&["Uranus", "Mars"]),
            ..ArbitrationAlertFilter::default()
        };
        assert!(planet.matches(&umbriel, &none));
        let planet_and_faction = ArbitrationAlertFilter {
            factions: set(&["Corpus"]),
            ..planet
        };
        assert!(!planet_and_faction.matches(&umbriel, &none));

        let mission = ArbitrationAlertFilter {
            mission_types: set(&["Interception"]),
            ..ArbitrationAlertFilter::default()
        };
        assert!(mission.matches(&umbriel, &none));
        let other_node = ArbitrationAlertFilter {
            node_ids: set(&["SolNode1"]),
            ..ArbitrationAlertFilter::default()
        };
        assert!(!other_node.matches(&umbriel, &none));

        let favourites_only = ArbitrationAlertFilter {
            favourites_only: true,
            ..ArbitrationAlertFilter::default()
        };
        assert!(!favourites_only.is_empty());
        assert!(favourites_only.matches(&umbriel, &favourites));
        assert!(!favourites_only.matches(&umbriel, &none));

        let unknown = arbitration_node("SolNode99999", &tiers());
        assert!(!planet_and_faction.matches(&unknown, &none));
        let lowest = ArbitrationAlertFilter {
            tiers: [ArbitrationTier::F].into_iter().collect(),
            ..ArbitrationAlertFilter::default()
        };
        assert!(lowest.matches(&unknown, &none));
        assert!(!lowest.matches(&umbriel, &none));
    }

    #[test]
    fn alerts_ahead_and_at_start() {
        let settings = ArbitrationAlertSettings {
            enabled: true,
            ..ArbitrationAlertSettings::default()
        };
        let umbriel_starts = FIRST_HOUR_MS + 2 * HOUR_MS;
        assert_eq!(
            alerts_at(&settings, umbriel_starts - 11 * MINUTE_MS),
            vec![]
        );
        assert_eq!(
            alerts_at(&settings, umbriel_starts - 10 * MINUTE_MS),
            vec![("1727892000:SolNode64:advance".to_owned(), false, 10, false)]
        );
        assert_eq!(
            alerts_at(&settings, umbriel_starts - 4 * MINUTE_MS - 1),
            vec![("1727892000:SolNode64:advance".to_owned(), false, 5, false)]
        );
        assert_eq!(
            alerts_at(&settings, umbriel_starts),
            vec![("1727892000:SolNode64:start".to_owned(), true, 0, false)]
        );
        assert_eq!(
            alerts_at(&settings, umbriel_starts + 5 * MINUTE_MS - 1),
            vec![("1727892000:SolNode64:start".to_owned(), true, 0, false)]
        );
        assert_eq!(alerts_at(&settings, umbriel_starts + 5 * MINUTE_MS), vec![]);
    }

    #[test]
    fn alerts_respect_lead_and_start_choices() {
        let at_start_only = ArbitrationAlertSettings {
            enabled: true,
            lead_minutes: 0,
            notify_on_start: false,
            ..ArbitrationAlertSettings::default()
        };
        let umbriel_starts = FIRST_HOUR_MS + 2 * HOUR_MS;
        assert_eq!(
            alerts_at(&at_start_only, umbriel_starts - MINUTE_MS),
            vec![]
        );
        assert_eq!(alerts_at(&at_start_only, umbriel_starts).len(), 1);

        let ahead_only = ArbitrationAlertSettings {
            enabled: true,
            lead_minutes: 60,
            notify_on_start: false,
            ..ArbitrationAlertSettings::default()
        };
        assert_eq!(
            alerts_at(&ahead_only, umbriel_starts - 60 * MINUTE_MS),
            vec![("1727892000:SolNode64:advance".to_owned(), false, 60, false)]
        );
        assert_eq!(
            alerts_at(&ahead_only, umbriel_starts),
            vec![("1727895600:SolNode305:advance".to_owned(), false, 60, false)]
        );

        let off = ArbitrationAlertSettings::default();
        assert_eq!(alerts_at(&off, umbriel_starts), vec![]);
        let filtered = ArbitrationAlertSettings {
            enabled: true,
            filter: ArbitrationAlertFilter {
                planets: set(&["Mars"]),
                ..ArbitrationAlertFilter::default()
            },
            ..ArbitrationAlertSettings::default()
        };
        assert_eq!(alerts_at(&filtered, umbriel_starts), vec![]);
    }

    #[test]
    fn reminders_fire_without_recurring_alerts() {
        let settings = ArbitrationAlertSettings {
            reminders: set(&["1727892000:SolNode64"]),
            ..ArbitrationAlertSettings::default()
        };
        let umbriel_starts = FIRST_HOUR_MS + 2 * HOUR_MS;
        assert_eq!(
            alerts_at(&settings, umbriel_starts - 30 * MINUTE_MS),
            vec![]
        );
        assert_eq!(
            alerts_at(&settings, umbriel_starts - 3 * MINUTE_MS),
            vec![("1727892000:SolNode64:advance".to_owned(), false, 3, true)]
        );
        assert_eq!(
            alerts_at(&settings, umbriel_starts + MINUTE_MS),
            vec![("1727892000:SolNode64:start".to_owned(), true, 0, true)]
        );
    }

    fn alert_settings() -> AlertSettings {
        AlertSettings {
            arbitrations: ArbitrationAlertSettings {
                enabled: true,
                ..ArbitrationAlertSettings::default()
            },
            ..AlertSettings::default()
        }
    }

    #[test]
    fn arbitration_alert_once() {
        let forecast = forecast();
        let tiers = tiers();
        let fetched_at = at(1_727_884_800_000);
        let starts = at(FIRST_HOUR_MS + 2 * HOUR_MS);
        let mut engine = Engine::new(alert_settings());

        let events = engine.handle_arbitrations(&forecast, &tiers, fetched_at, starts);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            CoreEvent::ArbitrationAlert { alert } if alert.at_start && alert.slot.node.node == "Umbriel"
        ));
        let again = starts + TimeDelta::minutes(1);
        assert!(
            engine
                .handle_arbitrations(&forecast, &tiers, fetched_at, again)
                .is_empty()
        );
    }

    #[test]
    fn stale_forecast_is_silent() {
        let forecast = forecast();
        let tiers = tiers();
        let starts = at(FIRST_HOUR_MS + 2 * HOUR_MS);
        let fetched_long_ago = starts - TimeDelta::hours(25);
        let mut engine = Engine::new(alert_settings());
        assert!(
            engine
                .handle_arbitrations(&forecast, &tiers, fetched_long_ago, starts)
                .is_empty()
        );
    }
}
