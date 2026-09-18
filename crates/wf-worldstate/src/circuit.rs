use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::mongo_date::MongoDate;

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
    pub activation: MongoDate,
    pub expiry: MongoDate,
    #[serde(default)]
    pub category_choices: Vec<CategoryChoices>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    let week = schedule
        .iter()
        .find(|week| now >= week.activation.utc() && now < week.expiry.utc())?;
    Some(Circuit {
        rotates: week.expiry.utc(),
        normal: choices(week, "EXC_NORMAL"),
        hard: choices(week, "EXC_HARD"),
    })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    fn week(activation_ms: i64, expiry_ms: i64) -> EndlessXpWeek {
        EndlessXpWeek {
            activation: DateTime::<Utc>::from_timestamp_millis(activation_ms)
                .unwrap()
                .into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(expiry_ms)
                .unwrap()
                .into(),
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
        let now = DateTime::<Utc>::from_timestamp_millis(1500).unwrap();
        let circuit = current_circuit(&[week(1000, 2000)], now).unwrap();
        assert_eq!(circuit.rotates.timestamp_millis(), 2000);
        assert_eq!(circuit.normal, vec!["Garuda".to_owned()]);
        assert_eq!(circuit.hard, vec!["Boar".to_owned(), "Anku".to_owned()]);
    }

    #[test]
    fn no_running_week() {
        let now = DateTime::<Utc>::from_timestamp_millis(2500).unwrap();
        assert!(current_circuit(&[week(1000, 2000), week(3000, 4000)], now).is_none());
    }
}
