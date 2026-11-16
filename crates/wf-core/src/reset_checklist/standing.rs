use chrono::{DateTime, Utc};
use wf_inventory::{DailyStandingPool, Inventory};
use wf_worldstate::ResetWindow;

use crate::reset_checklist::cadence::ResetCadence;
use crate::reset_checklist::context::{left_today, synced_at};

pub(crate) struct DailyStanding {
    pub(crate) reset: ResetWindow,
    pub(crate) cap: u32,
    left: Vec<(DailyStandingPool, u32)>,
}

impl DailyStanding {
    pub(crate) fn new(inventory: &Inventory, now: DateTime<Utc>) -> Self {
        let reset = ResetCadence::Daily.window(now);
        let cap = 16_000 + 500 * inventory.player_level;
        let synced_today = synced_at(inventory).is_some_and(|synced| reset.contains(synced));

        Self {
            reset,
            cap,
            left: inventory
                .daily_standing_remaining()
                .map(|(pool, left)| (pool, left_today(synced_today, left, cap)))
                .collect(),
        }
    }

    pub(crate) fn left(&self, pool: DailyStandingPool) -> Option<u32> {
        self.left
            .iter()
            .find(|&&(listed, _)| listed == pool)
            .map(|&(_, left)| left)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;
    use crate::catalog::fixtures;
    use crate::reset_checklist::context::fixtures::synced_at;

    #[test]
    fn today_and_stale() {
        let inventory = fixtures::inventory();

        let today = DailyStanding::new(&inventory, synced_at());
        let tomorrow = DailyStanding::new(&inventory, synced_at() + TimeDelta::days(1));

        assert_eq!(today.cap, 23_000);
        assert_eq!(today.left(DailyStandingPool::Factions), Some(21_000));
        assert_eq!(today.left(DailyStandingPool::Library), Some(0));
        assert_eq!(tomorrow.left(DailyStandingPool::Factions), Some(23_000));
        assert_eq!(tomorrow.left(DailyStandingPool::Library), Some(23_000));
    }
}
