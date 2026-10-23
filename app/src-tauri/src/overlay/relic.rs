use std::sync::Arc;

use tauri::{AppHandle, Runtime};
use tracing::debug;
use wf_core::tier_name;
use wf_scan::RelicScreen;
use wf_worldstate::RelicTier;

use super::screens::Game;
use super::{Kind, RecommendationTrigger, lua_read, show};
use crate::runtime::blocking;
use crate::settings::Settings;
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

fn recommendation_for(game: &Game, screen: Option<RelicScreen>) -> Recommendation {
    match screen {
        Some(RelicScreen::Era(era)) => Recommendation::of(RelicTier::from_modifier(era)),
        Some(RelicScreen::AnyEra) => Recommendation::AnyEra,
        Some(RelicScreen::Orbiter) => Recommendation::Hidden,
        None => Recommendation::of(game.mission_tier),
    }
}

fn read_relic_screen(state: &Arc<AppState>) -> Option<RelicScreen> {
    let screen = lua_read(state, wf_scan::relic_screen);
    debug!(?screen, "Relic screen read");
    screen
}

pub(super) fn scan_recommendation<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    generation: u64,
) {
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let scanning = Arc::clone(&owned);
        let screen = blocking("read relic screen", move || read_relic_screen(&scanning))
            .await
            .flatten();

        if lock(&owned.core).inventory().is_none() {
            debug!("No inventory yet, skipping relic recommendation");
            return;
        }
        let recommendation = recommendation_for(&lock(&owned.overlays.screens).game, screen);
        let tier = match recommendation {
            Recommendation::Hidden => return,
            Recommendation::AnyEra => None,
            Recommendation::Era(era) => Some(era),
        };
        if !lock(&owned.overlays.screens).recommendation_pending(generation) {
            return;
        }
        let trigger = recommendation_trigger(&read(&owned.settings), tier);
        show(&app, &owned, Kind::RelicRecommendation, |slots| {
            slots.recommendation = Some(trigger);
        });
    });
}

pub(super) fn recommendation_trigger(
    settings: &Settings,
    tier: Option<&'static str>,
) -> RecommendationTrigger {
    RecommendationTrigger {
        tier,
        refinement: settings.overlays.overlay_recommendation_refinement,
        count: settings.overlays.overlay_recommendation_count,
    }
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
    fn screen_over_mission_tier() {
        let mut game = Game::default();
        game.set_mission(Some(RelicTier::Neo));
        assert_eq!(
            recommendation_for(&game, Some(RelicScreen::Era("VoidT1"))),
            Recommendation::Era("Lith")
        );
        assert_eq!(
            recommendation_for(&game, Some(RelicScreen::Era("VoidT5"))),
            Recommendation::Hidden
        );
        assert_eq!(
            recommendation_for(&game, Some(RelicScreen::AnyEra)),
            Recommendation::AnyEra
        );
        assert_eq!(
            recommendation_for(&game, Some(RelicScreen::Orbiter)),
            Recommendation::Hidden
        );
        assert_eq!(recommendation_for(&game, None), Recommendation::Era("Neo"));

        game.set_mission(None);
        assert_eq!(recommendation_for(&game, None), Recommendation::Hidden);

        game.set_mission(Some(RelicTier::Requiem));
        assert!(game.requiem_mission());
        assert_eq!(recommendation_for(&game, None), Recommendation::Hidden);
    }
}
