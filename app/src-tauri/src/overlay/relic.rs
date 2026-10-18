use std::sync::Arc;

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Runtime};
use tracing::{debug, warn};
use wf_core::tier_name;
use wf_worldstate::{FissureTier, RelicTier, WorldState};

use super::screens::Game;
use super::{Kind, RecommendationTrigger, attached_game, show};
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

pub(super) fn suggest_mission(state: &AppState, node: &str) {
    if lock(&state.overlays.screens).game.mission_node.as_deref() == Some(node) {
        return;
    }
    let tier = lock(&state.core)
        .world()
        .and_then(|world| fissure_tier_at(world, node, Utc::now()));
    debug!(node, ?tier, "Squad mission suggested");
    lock(&state.overlays.screens)
        .game
        .set_mission(Some(node), tier);
}

fn fissure_tier_at(world: &WorldState, mission: &str, now: DateTime<Utc>) -> Option<RelicTier> {
    let node = mission.split('_').next()?;
    world
        .fissures(now)
        .into_iter()
        .find(|fissure| fissure.node_id == node)
        .and_then(|fissure| match fissure.tier {
            FissureTier::Relic(tier) => Some(tier),
            FissureTier::Unknown(_) => None,
        })
}

fn recommendation_for(game: &Game, scanned: Option<RelicTier>) -> Recommendation {
    if game.star_chart_open && scanned.is_some() {
        return Recommendation::of(scanned);
    }
    Recommendation::of(game.mission_tier)
}

fn selected_fissure_tier(state: &AppState) -> Option<RelicTier> {
    let reader = attached_game(state)?;
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
    generation: u64,
) {
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let from_star_chart = lock(&owned.overlays.screens).game.star_chart_open;
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
        let recommendation = recommendation_for(&lock(&owned.overlays.screens).game, scanned);
        let tier = match recommendation {
            Recommendation::Hidden => return,
            Recommendation::AnyEra => None,
            Recommendation::Era(era) => Some(era),
        };
        if !lock(&owned.overlays.screens).recommendation_pending(generation) {
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
    fn fissure_tier_of_suggested_mission() {
        let world =
            WorldState::parse(include_str!("../../../../fixtures/worldState.json")).unwrap();
        let now = DateTime::<Utc>::from_timestamp_millis(1_788_804_000_000).unwrap();
        assert_eq!(
            fissure_tier_at(&world, "SolNode215_ActiveMission", now),
            Some(RelicTier::Neo)
        );
        assert_eq!(
            fissure_tier_at(&world, "SolNode403_Hard", now),
            Some(RelicTier::Axi)
        );
        assert_eq!(fissure_tier_at(&world, "IceBladeHUB_HUB", now), None);
        let expired = DateTime::<Utc>::from_timestamp_millis(1_800_000_000_000).unwrap();
        assert_eq!(
            fissure_tier_at(&world, "SolNode215_ActiveMission", expired),
            None
        );
    }

    #[test]
    fn star_chart_vs_mission_tier() {
        let mut game = Game::default();
        game.star_chart_open = true;
        game.set_mission(None, Some(RelicTier::Neo));
        assert_eq!(
            recommendation_for(&game, Some(RelicTier::Lith)),
            Recommendation::Era("Lith")
        );
        assert_eq!(
            recommendation_for(&game, Some(RelicTier::Omnia)),
            Recommendation::AnyEra
        );
        assert_eq!(recommendation_for(&game, None), Recommendation::Era("Neo"));

        game.star_chart_open = false;
        assert_eq!(
            recommendation_for(&game, Some(RelicTier::Lith)),
            Recommendation::Era("Neo")
        );

        game.set_mission(None, None);
        assert_eq!(
            recommendation_for(&game, Some(RelicTier::Lith)),
            Recommendation::Hidden
        );

        game.set_mission(Some("SolNode1"), Some(RelicTier::Axi));
        assert_eq!(recommendation_for(&game, None), Recommendation::Era("Axi"));
        assert!(!game.requiem_mission());

        game.set_mission(Some("SolNode1"), Some(RelicTier::Requiem));
        assert!(game.requiem_mission());
        assert_eq!(recommendation_for(&game, None), Recommendation::Hidden);
    }
}
