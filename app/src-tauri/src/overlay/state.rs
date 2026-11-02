use serde::Serialize;
use wf_core::{ArbitrationRun, DucateringProgress, RewardScreen, RivenRow};

use super::{Kind, window_shows};
use crate::settings::{Motion, OVERLAY_OPACITY_DEFAULT, RecommendationRefinement, TextSize};
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RewardTrigger {
    pub relic: Option<String>,
    pub screen: RewardScreen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RecommendationTrigger {
    pub tier: Option<&'static str>,
    pub refinement: RecommendationRefinement,
    pub count: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RivenTrigger {
    pub item_type: String,
    pub before: Option<RivenRow>,
    pub linked: Option<RivenRow>,
    pub attribution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct NotificationTrigger {
    pub id: u64,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct OverlayState {
    pub seq: u64,
    pub opacity: u8,
    pub text_size: TextSize,
    pub motion: Motion,
    pub ducatering_clipboard: bool,
    pub reward: Option<RewardTrigger>,
    pub recommendation: Option<RecommendationTrigger>,
    pub riven: Option<RivenTrigger>,
    pub ducatering: Option<DucateringProgress>,
    pub notification: Option<NotificationTrigger>,
    pub arbitration: Option<Box<ArbitrationRun>>,
    pub preview: Option<Kind>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            seq: 0,
            opacity: OVERLAY_OPACITY_DEFAULT,
            text_size: TextSize::default(),
            motion: Motion::default(),
            ducatering_clipboard: true,
            reward: None,
            recommendation: None,
            riven: None,
            ducatering: None,
            notification: None,
            arbitration: None,
            preview: None,
        }
    }
}

impl OverlayState {
    pub(super) fn shows(&self, state: &AppState, only_while_game_active: bool, kind: Kind) -> bool {
        let focused = if self.preview == Some(kind) {
            None
        } else {
            state.focus.focused()
        };
        window_shows(only_while_game_active, focused, self.occupied(kind))
    }

    pub(super) fn occupied(&self, kind: Kind) -> bool {
        match kind {
            Kind::RelicReward => self.reward.is_some(),
            Kind::RelicRecommendation => self.recommendation.is_some(),
            Kind::Riven => self.riven.is_some(),
            Kind::Ducatering => self.ducatering.is_some(),
            Kind::Notification => self.notification.is_some(),
            Kind::ArbitrationLive => self.arbitration.is_some(),
        }
    }

    pub(super) fn clear(&mut self, kind: Kind) {
        self.preview.take_if(|preview| *preview == kind);
        match kind {
            Kind::RelicReward => self.reward = None,
            Kind::RelicRecommendation => self.recommendation = None,
            Kind::Riven => self.riven = None,
            Kind::Ducatering => self.ducatering = None,
            Kind::Notification => self.notification = None,
            Kind::ArbitrationLive => self.arbitration = None,
        }
    }
}
