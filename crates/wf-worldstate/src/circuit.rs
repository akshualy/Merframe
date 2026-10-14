use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::window::Window;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CategoryChoices {
    pub category: String,
    #[serde(default)]
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EndlessXpWeek {
    #[serde(flatten)]
    pub window: Window,
    #[serde(default)]
    pub category_choices: Vec<CategoryChoices>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct Circuit {
    pub rotates: DateTime<Utc>,
    pub normal: Vec<String>,
    pub hard: Vec<String>,
}

fn choices(week: &EndlessXpWeek, category: &str) -> Vec<String> {
    week.category_choices
        .iter()
        .filter(|entry| entry.category == category)
        .flat_map(|entry| entry.choices.iter().cloned())
        .collect()
}

pub fn current_circuit(schedule: &[EndlessXpWeek], now: DateTime<Utc>) -> Option<Circuit> {
    let week = schedule.iter().find(|week| week.window.contains(now))?;
    Some(Circuit {
        rotates: week.window.expiry.datetime(),
        normal: choices(week, "EXC_NORMAL"),
        hard: choices(week, "EXC_HARD"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::fixtures::{at, window};

    fn week(activation_ms: i64, expiry_ms: i64) -> EndlessXpWeek {
        EndlessXpWeek {
            window: window(activation_ms, expiry_ms),
            category_choices: vec![
                CategoryChoices {
                    category: "EXC_NORMAL".to_owned(),
                    choices: vec!["Garuda".to_owned()],
                },
                CategoryChoices {
                    category: "EXC_HARD".to_owned(),
                    choices: vec!["Boar".to_owned(), "Anku".to_owned()],
                },
            ],
        }
    }

    #[test]
    fn active_week_categories() {
        let now = at(1500);
        let circuit = current_circuit(&[week(1000, 2000)], now).unwrap();
        assert_eq!(circuit.rotates.timestamp_millis(), 2000);
        assert_eq!(circuit.normal, vec!["Garuda".to_owned()]);
        assert_eq!(circuit.hard, vec!["Boar".to_owned(), "Anku".to_owned()]);
    }

    #[test]
    fn no_running_week() {
        let now = at(2500);
        assert!(current_circuit(&[week(1000, 2000), week(3000, 4000)], now).is_none());
    }
}
