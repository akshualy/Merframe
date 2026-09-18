use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Runtime};
use tracing::{debug, warn};
use wf_core::tier_name;
use wf_worldstate::RelicTier;

use super::{Kind, Marks, RecommendationTrigger, attached_game, show, since};
use crate::runtime::blocking;
use crate::state::{AppState, lock, read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Recommendation {
    Hidden,
    AnyEra,
    Era(&'static str),
}

impl Recommendation {
    fn of(tier: Option<RelicTier>) -> Self {
        match tier {
            None | Some(RelicTier::Requiem) => Self::Hidden,
            Some(RelicTier::Omnia) => Self::AnyEra,
            Some(tier) => Self::Era(tier_name(tier)),
        }
    }
}

pub(super) fn recommendation_suppressed(marks: &Marks, at: f64) -> bool {
    since(marks.console, at).is_some_and(|elapsed| elapsed <= 1.0)
}

pub(super) fn recommendation_after_relic(marks: &Marks, at: f64) -> bool {
    since(marks.relic_rewards, at).is_some_and(|elapsed| elapsed < 23.0)
}

pub(super) fn recommendation_stays(marks: &Marks, at: f64) -> bool {
    since(marks.recommendation, at)
        .is_some_and(|elapsed| elapsed < 0.5 || (marks.after_relic && elapsed < 3.5))
}

pub(super) fn requiem_mission(marks: &Marks) -> bool {
    marks.mission_tier == Some(RelicTier::Requiem)
}

pub(super) fn reward_screen_closes(marks: &Marks, at: f64) -> bool {
    since(marks.relic_rewards, at).is_some_and(|elapsed| elapsed > 0.0)
}

fn opened_from_star_chart(marks: &Marks) -> bool {
    marks.star_chart > marks.star_chart_hidden
}

fn recommendation_for(marks: &Marks, scanned: Option<RelicTier>) -> Recommendation {
    if opened_from_star_chart(marks) && scanned.is_some() {
        return Recommendation::of(scanned);
    }
    Recommendation::of(marks.mission_tier)
}

fn selected_fissure_tier(state: &AppState) -> Option<RelicTier> {
    let reader = attached_game()?;
    let found = lock(&state.overlays.relic_picker).find(reader.as_ref());
    let picked = match found {
        Ok(picked) => picked?,
        Err(error) => {
            warn!(%error, "Fissure picker read failed");
            return None;
        }
    };
    debug!(
        node = picked.node,
        tier = picked.void_tier,
        "Fissure selected on star chart"
    );
    RelicTier::from_modifier(picked.void_tier)
}

pub(super) fn scan_recommendation<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    ticket: u64,
) {
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let from_star_chart = opened_from_star_chart(&lock(&owned.overlays.marks));
        let scanned = if from_star_chart {
            let scanning = Arc::clone(&owned);
            blocking("scan fissure picker", move || {
                selected_fissure_tier(&scanning)
            })
            .await
            .flatten()
        } else {
            None
        };
        if lock(&owned.core).inventory().is_none() {
            debug!("No inventory yet, skipping relic recommendation");
            return;
        }
        let recommendation = recommendation_for(&lock(&owned.overlays.marks), scanned);
        let tier = match recommendation {
            Recommendation::Hidden => return,
            Recommendation::AnyEra => None,
            Recommendation::Era(era) => Some(era),
        };
        if owned.overlays.recommendation_ticket.load(Ordering::Relaxed) != ticket {
            return;
        }
        let (refinement, count) = {
            let settings = read(&owned.settings);
            (
                settings.overlays.overlay_recommendation_refinement,
                settings.overlays.overlay_recommendation_count,
            )
        };
        show(&app, &owned, Kind::RelicRecommendation, |slots| {
            slots.recommendation = Some(RecommendationTrigger {
                tier,
                refinement,
                count,
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use wf_worldstate::RelicTier;

    #[test]
    fn recommendation_of_void_tier() {
        let of = |modifier| Recommendation::of(RelicTier::from_modifier(modifier));
        assert_eq!(of("VoidT1"), Recommendation::Era("Lith"));
        assert_eq!(of("VoidT4"), Recommendation::Era("Axi"));
        assert_eq!(of("VoidT6"), Recommendation::AnyEra);
        assert_eq!(of("VoidT5"), Recommendation::Hidden);
        assert_eq!(of("SolNode1"), Recommendation::Hidden);
        assert_eq!(Recommendation::of(None), Recommendation::Hidden);
    }

    #[test]
    fn reward_screen_closes_after_shown() {
        let shown = Marks {
            relic_rewards: Some(3683.5),
            ..Marks::default()
        };
        assert!(!reward_screen_closes(&shown, 3683.0));
        assert!(reward_screen_closes(&shown, 3698.5));
        assert!(!reward_screen_closes(&Marks::default(), 3698.5));
    }

    #[test]
    fn star_chart_vs_mission_tier() {
        let star_chart = Marks {
            star_chart: Some(10.0),
            star_chart_hidden: Some(5.0),
            mission_tier: Some(RelicTier::Neo),
            ..Marks::default()
        };
        assert!(opened_from_star_chart(&star_chart));
        assert_eq!(
            recommendation_for(&star_chart, Some(RelicTier::Lith)),
            Recommendation::Era("Lith")
        );
        assert_eq!(
            recommendation_for(&star_chart, Some(RelicTier::Omnia)),
            Recommendation::AnyEra
        );
        assert_eq!(
            recommendation_for(&star_chart, None),
            Recommendation::Era("Neo")
        );

        let in_mission = Marks {
            star_chart_hidden: Some(15.0),
            ..star_chart.clone()
        };
        assert!(!opened_from_star_chart(&in_mission));
        assert_eq!(
            recommendation_for(&in_mission, Some(RelicTier::Lith)),
            Recommendation::Era("Neo")
        );

        let orbiter_without_mission = Marks {
            mission_tier: None,
            ..in_mission.clone()
        };
        assert_eq!(
            recommendation_for(&orbiter_without_mission, Some(RelicTier::Lith)),
            Recommendation::Hidden
        );

        let lobby = Marks {
            mission_tier: Some(RelicTier::Axi),
            ..Marks::default()
        };
        assert!(!opened_from_star_chart(&lobby));
        assert_eq!(recommendation_for(&lobby, None), Recommendation::Era("Axi"));
        assert_eq!(
            recommendation_for(&Marks::default(), None),
            Recommendation::Hidden
        );

        let requiem = Marks {
            mission_tier: Some(RelicTier::Requiem),
            ..Marks::default()
        };
        assert!(requiem_mission(&requiem));
        assert!(!requiem_mission(&lobby));
        assert_eq!(recommendation_for(&requiem, None), Recommendation::Hidden);
    }

    #[test]
    fn console_suppression_window() {
        let mut marks = Marks::default();
        assert!(!recommendation_suppressed(&marks, 100.0));

        marks.console = Some(100.0);
        assert!(recommendation_suppressed(&marks, 100.0));
        assert!(recommendation_suppressed(&marks, 101.0));
        assert!(!recommendation_suppressed(&marks, 101.001));
    }

    #[test]
    fn after_relic_window() {
        let marks = Marks {
            relic_rewards: Some(1000.0),
            ..Marks::default()
        };
        assert!(recommendation_after_relic(&marks, 1000.0));
        assert!(recommendation_after_relic(&marks, 1022.9));
        assert!(!recommendation_after_relic(&marks, 1023.0));
        assert!(!recommendation_after_relic(&Marks::default(), 1000.0));
    }

    #[test]
    fn recommendation_self_close() {
        let mut marks = Marks {
            recommendation: Some(200.0),
            ..Marks::default()
        };
        assert!(recommendation_stays(&marks, 200.0));
        assert!(recommendation_stays(&marks, 200.4));
        assert!(!recommendation_stays(&marks, 200.5));
        assert!(!recommendation_stays(&marks, 260.0));

        marks.after_relic = true;
        assert!(recommendation_stays(&marks, 200.5));
        assert!(recommendation_stays(&marks, 203.4));
        assert!(!recommendation_stays(&marks, 203.5));

        assert!(!recommendation_stays(&Marks::default(), 203.5));
    }
}
