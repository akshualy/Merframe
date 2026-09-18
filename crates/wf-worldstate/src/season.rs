use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::embedded::{ChallengeText, NIGHTWAVE_CHALLENGES, lookup};
use crate::mongo_date::MongoDate;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SeasonChallenge {
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub challenge: String,
    #[serde(default)]
    pub daily: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SeasonInfo {
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub affiliation_tag: String,
    pub season: u32,
    pub phase: u32,
    #[serde(default)]
    pub active_challenges: Vec<SeasonChallenge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChallengeKind {
    Daily,
    Weekly,
    EliteWeekly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NightwaveChallenge {
    pub tag: String,
    pub name: String,
    pub description: Option<&'static str>,
    pub kind: ChallengeKind,
    pub activation: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NightwaveSeason {
    pub season: u32,
    pub phase: u32,
    pub affiliation_tag: String,
    pub name: String,
    pub activation: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
    pub challenges: Vec<NightwaveChallenge>,
}

fn split_words(camel: &str) -> String {
    let mut words = String::with_capacity(camel.len() + 8);
    let mut previous: Option<char> = None;
    for ch in camel.chars() {
        let boundary = match previous {
            Some(before) => {
                (ch.is_ascii_uppercase() && (before.is_lowercase() || before.is_ascii_digit()))
                    || (ch.is_ascii_digit() && before.is_alphabetic())
            }
            None => false,
        };
        if boundary {
            words.push(' ');
        }
        words.push(ch);
        previous = Some(ch);
    }
    words
}

fn challenge_kind(raw: &SeasonChallenge) -> ChallengeKind {
    if raw.daily {
        ChallengeKind::Daily
    } else if raw
        .challenge
        .starts_with("/Lotus/Types/Challenges/Seasons/WeeklyHard/")
    {
        ChallengeKind::EliteWeekly
    } else {
        ChallengeKind::Weekly
    }
}

fn challenge_text(tag: &str) -> Option<&'static ChallengeText> {
    lookup(NIGHTWAVE_CHALLENGES, &tag.to_lowercase())
}

fn derived_challenge_name(tag: &str) -> String {
    let leaf = tag.rsplit_once('/').map_or(tag, |(_, leaf)| leaf);
    let stem = leaf.strip_prefix("Season").unwrap_or(leaf);
    let stem = ["WeeklyHard", "Weekly", "Daily"]
        .into_iter()
        .find_map(|segment| stem.strip_prefix(segment))
        .unwrap_or(stem);
    let stem = stem.strip_prefix("Permanent").unwrap_or(stem);
    let stem = stem.trim_end_matches(|letter: char| letter.is_ascii_digit());
    let name = split_words(stem);
    if name.is_empty() {
        leaf.to_owned()
    } else {
        name
    }
}

fn season_name(affiliation_tag: &str, season: u32) -> String {
    let stem = affiliation_tag
        .strip_prefix("RadioLegion")
        .unwrap_or(affiliation_tag);
    let stem = stem.strip_suffix("Syndicate").unwrap_or(stem);
    if stem.is_empty() || stem.chars().all(|c| c.is_ascii_digit()) {
        return format!("Series {season}");
    }
    split_words(stem)
}

pub fn current_season(raw: &SeasonInfo, now: DateTime<Utc>) -> NightwaveSeason {
    let mut challenges: Vec<NightwaveChallenge> = raw
        .active_challenges
        .iter()
        .filter(|challenge| now >= challenge.activation.utc() && now < challenge.expiry.utc())
        .map(|challenge| NightwaveChallenge {
            tag: challenge.challenge.clone(),
            name: challenge_text(&challenge.challenge).map_or_else(
                || derived_challenge_name(&challenge.challenge),
                |text| text.title.to_owned(),
            ),
            description: challenge_text(&challenge.challenge).map(|text| text.description),
            kind: challenge_kind(challenge),
            activation: challenge.activation.utc(),
            expiry: challenge.expiry.utc(),
        })
        .collect();
    challenges.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.expiry.cmp(&right.expiry))
            .then_with(|| left.name.cmp(&right.name))
    });
    NightwaveSeason {
        season: raw.season,
        phase: raw.phase,
        affiliation_tag: raw.affiliation_tag.clone(),
        name: season_name(&raw.affiliation_tag, raw.season),
        activation: raw.activation.utc(),
        expiry: raw.expiry.utc(),
        challenges,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    fn at(millis: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp_millis(millis).unwrap()
    }

    fn challenge(tag: &str, daily: bool, activation_ms: i64, expiry_ms: i64) -> SeasonChallenge {
        SeasonChallenge {
            activation: at(activation_ms).into(),
            expiry: at(expiry_ms).into(),
            challenge: tag.to_owned(),
            daily,
        }
    }

    fn season(challenges: Vec<SeasonChallenge>) -> SeasonInfo {
        SeasonInfo {
            activation: at(0).into(),
            expiry: at(10_000).into(),
            affiliation_tag: "RadioLegionIntermission16Syndicate".to_owned(),
            season: 18,
            phase: 0,
            active_challenges: challenges,
        }
    }

    #[test]
    fn running_challenges_only() {
        let raw = season(vec![
            challenge(
                "/Lotus/Types/Challenges/Seasons/Daily/SeasonDailyMercyKill",
                true,
                0,
                2000,
            ),
            challenge(
                "/Lotus/Types/Challenges/Seasons/Weekly/SeasonWeeklyNightAndDay",
                false,
                5000,
                6000,
            ),
        ]);
        let current = current_season(&raw, at(1000));
        assert_eq!(current.challenges.len(), 1);
        assert_eq!(current.challenges[0].name, "No Mercy");
        assert_eq!(current.challenges[0].kind, ChallengeKind::Daily);
        assert_eq!(
            current.challenges[0].description,
            Some("Mercy Kill an Enemy")
        );
    }

    #[test]
    fn challenge_order() {
        let raw = season(vec![
            challenge(
                "/Lotus/Types/Challenges/Seasons/WeeklyHard/SeasonWeeklyHardFallenAngel",
                false,
                0,
                2000,
            ),
            challenge(
                "/Lotus/Types/Challenges/Seasons/Weekly/SeasonWeeklyCompleteInvasionMissions",
                false,
                0,
                2000,
            ),
            challenge(
                "/Lotus/Types/Challenges/Seasons/Daily/SeasonDailyMercyKill",
                true,
                0,
                2000,
            ),
        ]);
        let kinds: Vec<_> = current_season(&raw, at(1000))
            .challenges
            .iter()
            .map(|challenge| challenge.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                ChallengeKind::Daily,
                ChallengeKind::Weekly,
                ChallengeKind::EliteWeekly
            ]
        );
    }

    #[test]
    fn table_titles() {
        let raw = season(vec![
            challenge(
                "/Lotus/Types/Challenges/Seasons/Daily/SeasonDailyKillEnemiesWithPoison",
                true,
                0,
                2000,
            ),
            challenge(
                "/Lotus/Types/Challenges/Seasons/WeeklyHard/SeasonWeeklyHardBattleHardened",
                false,
                0,
                2000,
            ),
        ]);
        let current = current_season(&raw, at(1000));
        assert_eq!(current.challenges[0].name, "Poisoner");
        assert_eq!(
            current.challenges[0].description,
            Some("Kill 150 Enemies with Toxin Damage")
        );
        assert_eq!(current.challenges[1].name, "Battle Hardened");
        assert_eq!(current.challenges[1].description, None);
    }

    #[test]
    fn embedded_table_sorted() {
        assert!(
            NIGHTWAVE_CHALLENGES
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0)
        );
        assert!(NIGHTWAVE_CHALLENGES.len() > 200);
    }

    #[test]
    fn derived_challenge_names() {
        assert_eq!(
            derived_challenge_name(
                "/Lotus/Types/Challenges/Seasons/Daily/SeasonDailyKillEnemiesWithPoison"
            ),
            "Kill Enemies With Poison"
        );
        assert_eq!(
            derived_challenge_name(
                "/Lotus/Types/Challenges/Seasons/Weekly/SeasonWeeklyPermanentCompleteMissions5"
            ),
            "Complete Missions"
        );
        assert_eq!(
            derived_challenge_name(
                "/Lotus/Types/Challenges/Seasons/WeeklyHard/SeasonWeeklyHardBattleHardened"
            ),
            "Battle Hardened"
        );
    }

    #[test]
    fn wordless_tag_keeps_leaf() {
        assert_eq!(
            derived_challenge_name("/Lotus/Types/Challenges/Seasons/Daily/SeasonDaily"),
            "SeasonDaily"
        );
    }

    #[test]
    fn season_names() {
        assert_eq!(
            season_name("RadioLegionIntermission16Syndicate", 18),
            "Intermission 16"
        );
        assert_eq!(season_name("RadioLegionSyndicate", 1), "Series 1");
        assert_eq!(season_name("RadioLegion3Syndicate", 12), "Series 12");
    }
}
