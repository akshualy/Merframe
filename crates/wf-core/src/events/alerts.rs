use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use wf_worldstate::{Fissure, RelicTier, mission_type_name};

const ANY: &str = "all";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FissureSubtype {
    #[default]
    All,
    Normal,
    SteelPath,
    VoidStorm,
}

impl FissureSubtype {
    pub fn matches(self, fissure: &Fissure) -> bool {
        match self {
            Self::All => true,
            Self::Normal => !fissure.steel_path && !fissure.is_storm,
            Self::SteelPath => fissure.steel_path,
            Self::VoidStorm => fissure.is_storm,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FissureFilter {
    pub tier: String,
    pub mission: String,
    pub location: String,
    #[serde(alias = "steel_path")]
    pub subtype: FissureSubtype,
}

impl Default for FissureFilter {
    fn default() -> Self {
        Self {
            tier: ANY.to_owned(),
            mission: ANY.to_owned(),
            location: ANY.to_owned(),
            subtype: FissureSubtype::All,
        }
    }
}

impl FissureFilter {
    pub fn matches(&self, fissure: &Fissure) -> bool {
        matches_value(&self.tier, fissure.tier.name())
            && self.matches_mission(fissure)
            && self.matches_location(fissure)
            && self.subtype.matches(fissure)
    }

    fn matches_mission(&self, fissure: &Fissure) -> bool {
        matches_value(&self.mission, &fissure.mission_type)
            || mission_type_name(&self.mission) == fissure.mission_name
    }

    fn matches_location(&self, fissure: &Fissure) -> bool {
        if self.location.eq_ignore_ascii_case(ANY) {
            return true;
        }
        fissure_planet(fissure).is_some_and(|planet| self.location.eq_ignore_ascii_case(planet))
    }
}

fn matches_value(filter: &str, value: &str) -> bool {
    filter.eq_ignore_ascii_case(ANY) || filter.eq_ignore_ascii_case(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CyclePhase {
    EarthDay,
    EarthNight,
    CetusDay,
    CetusNight,
    VallisWarm,
    VallisCold,
    CambionFass,
    CambionVome,
    DuviriSorrow,
    DuviriFear,
    DuviriJoy,
    DuviriAnger,
    DuviriEnvy,
    ZarimanCorpus,
    ZarimanGrineer,
}

impl CyclePhase {
    pub fn of(timer: &str, phase: &str) -> Option<Self> {
        Some(match (timer, phase) {
            ("Earth", "day") => Self::EarthDay,
            ("Earth", "night") => Self::EarthNight,
            ("Cetus", "day") => Self::CetusDay,
            ("Cetus", "night") => Self::CetusNight,
            ("Orb Vallis", "warm") => Self::VallisWarm,
            ("Orb Vallis", "cold") => Self::VallisCold,
            ("Cambion Drift", "fass") => Self::CambionFass,
            ("Cambion Drift", "vome") => Self::CambionVome,
            ("Duviri", "sorrow") => Self::DuviriSorrow,
            ("Duviri", "fear") => Self::DuviriFear,
            ("Duviri", "joy") => Self::DuviriJoy,
            ("Duviri", "anger") => Self::DuviriAnger,
            ("Duviri", "envy") => Self::DuviriEnvy,
            ("Zariman", "corpus") => Self::ZarimanCorpus,
            ("Zariman", "grineer") => Self::ZarimanGrineer,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(transparent)]
pub struct TimerAlerts(BTreeSet<CyclePhase>);

impl TimerAlerts {
    pub fn enabled(&self, timer: &str, phase: &str) -> bool {
        CyclePhase::of(timer, phase).is_some_and(|phase| self.0.contains(&phase))
    }
}

impl FromIterator<CyclePhase> for TimerAlerts {
    fn from_iter<I: IntoIterator<Item = CyclePhase>>(phases: I) -> Self {
        Self(phases.into_iter().collect())
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredTimerAlerts {
    Phases(BTreeSet<CyclePhase>),
    Toggles {
        #[serde(default)]
        earth: bool,
        #[serde(default)]
        cetus: bool,
        #[serde(default)]
        vallis: bool,
        #[serde(default)]
        cambion: bool,
        #[serde(flatten)]
        phases: BTreeMap<CyclePhase, bool>,
    },
}

impl<'de> Deserialize<'de> for TimerAlerts {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match StoredTimerAlerts::deserialize(deserializer)? {
            StoredTimerAlerts::Phases(phases) => Self(phases),
            StoredTimerAlerts::Toggles {
                earth,
                cetus,
                vallis,
                cambion,
                phases,
            } => {
                let mut set = BTreeSet::new();
                let worlds = [
                    (earth, [CyclePhase::EarthDay, CyclePhase::EarthNight]),
                    (cetus, [CyclePhase::CetusDay, CyclePhase::CetusNight]),
                    (vallis, [CyclePhase::VallisWarm, CyclePhase::VallisCold]),
                    (cambion, [CyclePhase::CambionFass, CyclePhase::CambionVome]),
                ];
                for (on, both) in worlds {
                    if on {
                        set.extend(both);
                    }
                }
                for (phase, on) in phases {
                    if on {
                        set.insert(phase);
                    } else {
                        set.remove(&phase);
                    }
                }
                Self(set)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertSettings {
    pub fissure_notifications_enabled: bool,
    pub fissure_filters: Vec<FissureFilter>,
    pub timers: TimerAlerts,
    pub timer_lead_secs: i64,
}

impl Default for AlertSettings {
    fn default() -> Self {
        Self {
            fissure_notifications_enabled: false,
            fissure_filters: Vec::new(),
            timers: TimerAlerts::default(),
            timer_lead_secs: 180,
        }
    }
}

impl AlertSettings {
    pub fn wants(&self, fissure: &Fissure) -> bool {
        self.fissure_notifications_enabled
            && self
                .fissure_filters
                .iter()
                .any(|filter| filter.matches(fissure))
    }
}

pub(super) fn fissure_planet(fissure: &Fissure) -> Option<&'static str> {
    let name = fissure.node_name?.trim_end();
    let inner = name.strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let planet = inner[open + 1..].trim();
    (!planet.is_empty()).then_some(planet)
}

pub fn tier_name(tier: RelicTier) -> &'static str {
    tier.name()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use wf_worldstate::FissureTier;

    fn at(millis: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(millis).unwrap()
    }

    fn fissure(
        tier: RelicTier,
        mission_type: &str,
        node_name: Option<&'static str>,
        steel_path: bool,
    ) -> Fissure {
        Fissure {
            node_id: "SolNode58".to_owned(),
            node_name,
            mission_type: mission_type.to_owned(),
            mission_name: "Extermination".to_owned(),
            tier: FissureTier::Relic(tier),
            steel_path,
            is_storm: false,
            activation: at(0),
            expiry: at(1_000_000),
        }
    }

    fn sample() -> Fissure {
        fissure(
            RelicTier::Lith,
            "MT_EXTERMINATION",
            Some("Galatea (Neptune)"),
            false,
        )
    }

    #[test]
    fn planet_from_node_name() {
        assert_eq!(fissure_planet(&sample()), Some("Neptune"));
        assert_eq!(
            fissure_planet(&fissure(
                RelicTier::Lith,
                "MT_EXTERMINATION",
                Some("Galatea"),
                false
            )),
            None
        );
        assert_eq!(
            fissure_planet(&fissure(
                RelicTier::Lith,
                "MT_EXTERMINATION",
                Some("Kuva Fortress ()"),
                false
            )),
            None
        );
        assert_eq!(
            fissure_planet(&fissure(RelicTier::Lith, "MT_EXTERMINATION", None, false)),
            None
        );
    }

    #[test]
    fn default_filter_matches_all() {
        let filter = FissureFilter::default();
        assert!(filter.matches(&sample()));
        assert!(filter.matches(&fissure(RelicTier::Requiem, "MT_SURVIVAL", None, true)));
    }

    #[test]
    fn one_field_off() {
        let tier = FissureFilter {
            tier: "Axi".to_owned(),
            ..FissureFilter::default()
        };
        assert!(!tier.matches(&sample()));

        let mission = FissureFilter {
            mission: "MT_SURVIVAL".to_owned(),
            ..FissureFilter::default()
        };
        assert!(!mission.matches(&sample()));

        let location = FissureFilter {
            location: "Earth".to_owned(),
            ..FissureFilter::default()
        };
        assert!(!location.matches(&sample()));

        let steel_path = FissureFilter {
            subtype: FissureSubtype::SteelPath,
            ..FissureFilter::default()
        };
        assert!(!steel_path.matches(&sample()));
    }

    #[test]
    fn case_insensitive_fields() {
        let filter = FissureFilter {
            tier: "lith".to_owned(),
            mission: "mt_extermination".to_owned(),
            location: "neptune".to_owned(),
            subtype: FissureSubtype::Normal,
        };
        assert!(filter.matches(&sample()));
    }

    fn storm(mission: &str, node_name: &'static str) -> Fissure {
        Fissure {
            node_id: "CrewBattleNode531".to_owned(),
            node_name: Some(node_name),
            mission_type: mission.to_owned(),
            mission_name: mission.to_owned(),
            is_storm: true,
            ..sample()
        }
    }

    #[test]
    fn void_storm_filter() {
        let survival = storm("Survival", "Fenton's Field (Pluto)");
        let row = |subtype| FissureFilter {
            subtype,
            ..FissureFilter::default()
        };
        assert!(row(FissureSubtype::All).matches(&survival));
        assert!(row(FissureSubtype::All).matches(&sample()));
        assert!(row(FissureSubtype::VoidStorm).matches(&survival));
        assert!(!row(FissureSubtype::VoidStorm).matches(&sample()));
        assert!(!row(FissureSubtype::Normal).matches(&survival));
        assert!(!row(FissureSubtype::SteelPath).matches(&survival));
        assert!(row(FissureSubtype::Normal).matches(&sample()));
    }

    #[test]
    fn void_storm_mission_and_location() {
        let survival = storm("Survival", "Fenton's Field (Pluto)");
        let skirmish = storm("Skirmish", "Gian Point (Veil)");
        let row = |mission: &str, location: &str| FissureFilter {
            mission: mission.to_owned(),
            location: location.to_owned(),
            subtype: FissureSubtype::VoidStorm,
            ..FissureFilter::default()
        };
        assert!(row("MT_SURVIVAL", "Pluto").matches(&survival));
        assert!(!row("MT_SURVIVAL", "Veil").matches(&survival));
        assert!(!row("MT_INTEL", "all").matches(&survival));
        assert!(row("Skirmish", "Veil").matches(&skirmish));
        assert!(!row("Skirmish", "all").matches(&survival));
    }

    #[test]
    fn stored_steel_path_row() {
        let row: FissureFilter = serde_json::from_str(r#"{"steel_path":"steelPath"}"#).unwrap();
        assert_eq!(row.subtype, FissureSubtype::SteelPath);
        let row: FissureFilter = serde_json::from_str(r#"{"subtype":"voidStorm"}"#).unwrap();
        assert_eq!(row.subtype, FissureSubtype::VoidStorm);
    }

    #[test]
    fn any_row_matches() {
        let settings = AlertSettings {
            fissure_notifications_enabled: true,
            fissure_filters: vec![
                FissureFilter {
                    tier: "Axi".to_owned(),
                    ..FissureFilter::default()
                },
                FissureFilter {
                    tier: "Lith".to_owned(),
                    ..FissureFilter::default()
                },
            ],
            ..AlertSettings::default()
        };
        assert!(settings.wants(&sample()));
    }

    #[test]
    fn no_rows_no_alerts() {
        let settings = AlertSettings {
            fissure_notifications_enabled: true,
            ..AlertSettings::default()
        };
        assert!(!settings.wants(&sample()));
    }

    #[test]
    fn legacy_world_toggles() {
        let parsed: AlertSettings = serde_json::from_str(
            r#"{"timers":{"earth":true,"cetus":false,"vallis":true,"cambion":false}}"#,
        )
        .unwrap();
        assert_eq!(
            parsed.timers,
            TimerAlerts::from_iter([
                CyclePhase::EarthDay,
                CyclePhase::EarthNight,
                CyclePhase::VallisWarm,
                CyclePhase::VallisCold,
            ])
        );
    }

    #[test]
    fn phase_toggle_beats_world_toggle() {
        let parsed: AlertSettings =
            serde_json::from_str(r#"{"timers":{"cetus":true,"cetus_day":false}}"#).unwrap();
        assert_eq!(
            parsed.timers,
            TimerAlerts::from_iter([CyclePhase::CetusNight])
        );
    }

    #[test]
    fn phase_toggle_map() {
        let parsed: AlertSettings = serde_json::from_str(
            r#"{"timers":{
                "earth_day":true,"earth_night":false,"cetus_day":false,"cetus_night":true,
                "vallis_warm":false,"vallis_cold":false,"cambion_fass":true,"cambion_vome":false,
                "duviri_sorrow":false,"duviri_fear":false,"duviri_joy":true,"duviri_anger":false,
                "duviri_envy":false,"zariman_corpus":false,"zariman_grineer":true}}"#,
        )
        .unwrap();
        assert_eq!(
            parsed.timers,
            TimerAlerts::from_iter([
                CyclePhase::EarthDay,
                CyclePhase::CetusNight,
                CyclePhase::CambionFass,
                CyclePhase::DuviriJoy,
                CyclePhase::ZarimanGrineer,
            ])
        );
        let empty: AlertSettings = serde_json::from_str(r#"{"timers":{}}"#).unwrap();
        assert_eq!(empty.timers, TimerAlerts::default());
    }

    #[test]
    fn settings_json_round_trip() {
        let settings = AlertSettings {
            fissure_notifications_enabled: true,
            fissure_filters: vec![FissureFilter {
                tier: "Lith".to_owned(),
                mission: "MT_EXTERMINATION".to_owned(),
                location: "Earth".to_owned(),
                subtype: FissureSubtype::SteelPath,
            }],
            timers: TimerAlerts::from_iter([
                CyclePhase::EarthDay,
                CyclePhase::CetusNight,
                CyclePhase::VallisWarm,
                CyclePhase::DuviriJoy,
                CyclePhase::ZarimanGrineer,
            ]),
            timer_lead_secs: 300,
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains(r#""subtype":"steelPath""#));
        assert!(json.contains(
            r#""timers":["earth_day","cetus_night","vallis_warm","duviri_joy","zariman_grineer"]"#
        ));
        assert!(json.contains(r#""fissure_notifications_enabled":true"#));
        let parsed: AlertSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, settings);
    }

    #[test]
    fn partial_settings() {
        let parsed: AlertSettings = serde_json::from_str(r#"{"timer_lead_secs":60}"#).unwrap();
        assert_eq!(
            parsed,
            AlertSettings {
                timer_lead_secs: 60,
                ..AlertSettings::default()
            }
        );
        let row: FissureFilter = serde_json::from_str(r#"{"tier":"Axi"}"#).unwrap();
        assert_eq!(
            row,
            FissureFilter {
                tier: "Axi".to_owned(),
                ..FissureFilter::default()
            }
        );
    }
}
