use serde::Serialize;
use wf_data::{Refinement, Relic};

use crate::catalog::{REFINEMENTS, refinement_name};

use super::RewardBreakdown;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RefinementValue {
    pub refinement: &'static str,
    pub expected_plat: f64,
    pub expected_ducats: f64,
    pub wanted_chance: f64,
    pub traces: u32,
    pub plat_per_trace: Option<f64>,
    pub ducats_per_trace: Option<f64>,
    pub chances: Vec<f64>,
    pub expected_plat_shares: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PerTrace {
    pub value: f64,
    pub refinement: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Best {
    pub plat: f64,
    pub refinement: &'static str,
    pub wanted_chance: f64,
    pub plat_per_trace: Option<PerTrace>,
    pub ducats_per_trace: Option<PerTrace>,
}

pub(super) fn best_refinement(values: &[RefinementValue]) -> Best {
    let mut best = Best {
        plat: 0.0,
        refinement: refinement_name(Refinement::Intact),
        wanted_chance: 0.0,
        plat_per_trace: None,
        ducats_per_trace: None,
    };
    for value in values {
        if value.expected_plat > best.plat {
            best.plat = value.expected_plat;
            best.refinement = value.refinement;
        }
        best.wanted_chance = best.wanted_chance.max(value.wanted_chance);
        raise(
            &mut best.plat_per_trace,
            value.plat_per_trace,
            value.refinement,
        );
        raise(
            &mut best.ducats_per_trace,
            value.ducats_per_trace,
            value.refinement,
        );
    }
    best
}

fn raise(best: &mut Option<PerTrace>, value: Option<f64>, refinement: &'static str) {
    if value > best.map(|best| best.value) {
        *best = value.map(|value| PerTrace { value, refinement });
    }
}

fn best_of_squad_shares(outcomes: &[(f64, f64)], squad_size: u32) -> Vec<f64> {
    let exponent = f64::from(squad_size);
    let mut order: Vec<usize> = (0..outcomes.len()).collect();
    order.sort_by(|&left, &right| outcomes[left].0.total_cmp(&outcomes[right].0));
    let mut shares = vec![0.0; outcomes.len()];
    let mut below = 0.0;
    let mut index = 0;
    while index < order.len() {
        let value = outcomes[order[index]].0;
        let group = index;
        let mut cumulative = below;
        while index < order.len() && outcomes[order[index]].0 <= value {
            cumulative += outcomes[order[index]].1;
            index += 1;
        }
        let expected = value * (cumulative.powf(exponent) - below.powf(exponent));
        let weight: f64 = order[group..index]
            .iter()
            .map(|&entry| outcomes[entry].1)
            .sum();
        for &entry in &order[group..index] {
            shares[entry] = expected * outcomes[entry].1 / weight;
        }
        below = cumulative;
    }
    shares
}

fn trace_cost(refinement: Refinement) -> u32 {
    match refinement {
        Refinement::Intact => 0,
        Refinement::Exceptional => 25,
        Refinement::Flawless => 50,
        Refinement::Radiant => 100,
    }
}

#[allow(
    clippy::cast_possible_wrap,
    reason = "a squad has at most four members"
)]
pub(super) fn refinement_values(
    relic: &Relic,
    rewards: &[RewardBreakdown],
    wanted_rewards: &[bool],
    squad_size: u32,
) -> [RefinementValue; 4] {
    let mut values = REFINEMENTS.map(|refinement| {
        let chances: Vec<f64> = relic
            .rewards_for(refinement)
            .iter()
            .map(|reward| reward.chance)
            .collect();
        let plat_outcomes: Vec<(f64, f64)> = rewards
            .iter()
            .zip(&chances)
            .map(|(reward, chance)| (reward.plat.unwrap_or_default(), chance / 100.0))
            .collect();
        let ducat_outcomes: Vec<(f64, f64)> = rewards
            .iter()
            .zip(&chances)
            .map(|(reward, chance)| (f64::from(reward.ducats.unwrap_or_default()), chance / 100.0))
            .collect();
        let expected_plat_shares = best_of_squad_shares(&plat_outcomes, squad_size);
        let expected_ducats: f64 = best_of_squad_shares(&ducat_outcomes, squad_size)
            .iter()
            .sum();
        let missed: f64 = wanted_rewards
            .iter()
            .zip(&chances)
            .filter(|(wanted, _)| **wanted)
            .map(|(_, chance)| (1.0 - chance / 100.0).powi(squad_size as i32))
            .product();
        RefinementValue {
            refinement: refinement_name(refinement),
            expected_plat: expected_plat_shares.iter().sum(),
            expected_ducats,
            wanted_chance: (1.0 - missed) * 100.0,
            traces: trace_cost(refinement),
            plat_per_trace: None,
            ducats_per_trace: None,
            chances,
            expected_plat_shares,
        }
    });
    let intact_plat = values[0].expected_plat;
    let intact_ducats = values[0].expected_ducats;
    for value in &mut values {
        if value.traces > 0 {
            let traces = f64::from(value.traces);
            value.plat_per_trace = Some((value.expected_plat - intact_plat) / traces);
            value.ducats_per_trace = Some((value.expected_ducats - intact_ducats) / traces);
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_squad() {
        let outcomes = [(10.0, 0.25), (20.0, 0.25), (5.0, 0.5)];
        let solo: f64 = best_of_squad_shares(&outcomes, 1).iter().sum();
        assert!((solo - 10.0).abs() < 1e-9);
        let pair: f64 = best_of_squad_shares(&outcomes, 2).iter().sum();
        assert!(pair > solo);
    }

    #[test]
    fn shares_by_weight() {
        let outcomes = [(10.0, 0.25), (20.0, 0.25), (5.0, 0.5)];
        let solo = best_of_squad_shares(&outcomes, 1);
        assert!((solo[0] - 2.5).abs() < 1e-9);
        assert!((solo[1] - 5.0).abs() < 1e-9);
        assert!((solo[2] - 2.5).abs() < 1e-9);

        let tied = best_of_squad_shares(&[(7.0, 0.25), (7.0, 0.75)], 1);
        assert!((tied[0] - 1.75).abs() < 1e-9);
        assert!((tied[1] - 5.25).abs() < 1e-9);
    }
}
