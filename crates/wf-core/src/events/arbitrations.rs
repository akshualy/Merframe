use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;
use wf_worldstate::{
    ArbitrationForecast, ArbitrationForecastHour, ArbitrationNode, ArbitrationTiers,
    arbitration_node,
};

pub const ARBITRATION_FORECAST_TRUSTED: TimeDelta = TimeDelta::hours(24);
const SCHEDULE_HOURS: usize = 169;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ArbitrationSlot {
    pub id: String,
    #[serde(flatten)]
    pub node: ArbitrationNode,
    pub starts: DateTime<Utc>,
    pub ends: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ArbitrationSchedule {
    pub slots: Vec<ArbitrationSlot>,
    pub nodes: Vec<ArbitrationNode>,
}

pub(crate) struct ArbitrationOutlook<'a> {
    pub forecast: &'a ArbitrationForecast,
    pub tiers: &'a ArbitrationTiers,
}

impl ArbitrationOutlook<'_> {
    fn slot(&self, hour: ArbitrationForecastHour<'_>) -> ArbitrationSlot {
        ArbitrationSlot {
            id: format!("{}:{}", hour.starts.timestamp(), hour.node_id),
            node: arbitration_node(hour.node_id, self.tiers),
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
        Some(ArbitrationSchedule { slots, nodes })
    }
}

#[cfg(test)]
mod tests {
    use wf_worldstate::ArbitrationTier;

    use super::*;
    use crate::catalog::fixtures::{self, at};

    const FIRST_HOUR_MS: i64 = 1_727_884_800_000;
    const HOUR_MS: i64 = 3_600_000;

    fn schedule_at(now_ms: i64) -> Option<ArbitrationSchedule> {
        let forecast = ArbitrationForecast::parse(fixtures::ARBITRATIONS).unwrap();
        let tiers = ArbitrationTiers::parse(fixtures::ARBITRATION_TIERS).unwrap();
        ArbitrationOutlook {
            forecast: &forecast,
            tiers: &tiers,
        }
        .schedule(at(now_ms))
    }

    #[test]
    fn schedule_runs_from_the_current_hour() {
        let schedule = schedule_at(FIRST_HOUR_MS + 2 * HOUR_MS + 1).unwrap();
        assert_eq!(schedule.slots.len(), 46);
        assert_eq!(schedule.slots[0].id, "1727892000:SolNode64");
        assert_eq!(schedule.slots[0].node.node, "Umbriel");
        assert_eq!(schedule.slots[0].node.planet, Some("Uranus"));
        assert_eq!(schedule.slots[0].node.tier, ArbitrationTier::S);
        assert_eq!(schedule.slots[0].starts, at(FIRST_HOUR_MS + 2 * HOUR_MS));
        assert_eq!(schedule.slots[0].ends, at(FIRST_HOUR_MS + 3 * HOUR_MS));
        assert_eq!(schedule.nodes.len(), 38);
        assert!(
            schedule
                .nodes
                .is_sorted_by_key(|node| (node.planet, node.node.clone()))
        );
        assert_eq!(schedule_at(FIRST_HOUR_MS - 1), None);
    }
}
