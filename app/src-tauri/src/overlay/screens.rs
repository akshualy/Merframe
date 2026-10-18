use wf_core::RivenRow;
use wf_worldstate::RelicTier;

const RELIC_SELECT_FROM_CONSOLE: f64 = 1.0;
const ENDLESS_FISSURE_NEXT_RELIC: f64 = 23.0;
const RESETS_WHILE_LOADING: f64 = 0.5;
const RESETS_WHILE_LOADING_AFTER_REWARDS: f64 = 3.5;
const RIVEN_LINE_AFTER_POPUP: f64 = 2.0;
const RIVEN_LINE_REPEAT: f64 = 0.4;
const CYCLE_ANSWER_TIMEOUT: f64 = 300.0;

#[derive(Debug, Default)]
pub(super) struct Screens {
    pub(super) game: Game,
    pub(super) reward: Reward,
    pub(super) recommendation: Recommendation,
    pub(super) chat_link: ChatLink,
    pub(super) cycling: Cycling,
    generation: u64,
}

impl Screens {
    fn next_generation(&mut self) -> u64 {
        self.generation += 1;
        self.generation
    }

    pub(super) fn relic_select_loaded(&mut self, at: f64) -> Option<u64> {
        if self.game.console_just_opened(at) {
            return None;
        }

        let generation = self.next_generation();
        self.recommendation = Recommendation::Open {
            at,
            after_relic: self.reward.just_shown(at),
            generation,
        };
        Some(generation)
    }

    pub(super) fn reward_shows(&self) -> bool {
        !self.game.requiem_mission() && self.reward.is_open()
    }

    pub(super) fn recommendation_pending(&self, generation: u64) -> bool {
        matches!(self.recommendation, Recommendation::Open { generation: open, .. } if open == generation)
    }

    pub(super) fn chat_link_layers(&mut self, count: u32, at: f64) -> bool {
        let dropped = count < self.game.hud_layers;
        self.game.hud_layers = count;
        if dropped {
            self.chat_link = ChatLink::Closed;
            return true;
        }

        self.chat_link = match self.chat_link {
            ChatLink::Open { line, .. } => ChatLink::Open { since: at, line },
            ChatLink::Closed => ChatLink::Open {
                since: at,
                line: None,
            },
        };
        false
    }

    pub(super) fn riven_line(&mut self, at: f64) -> Option<u64> {
        let ChatLink::Open { since, line } = self.chat_link else {
            return None;
        };
        let in_window = (0.0..RIVEN_LINE_AFTER_POPUP).contains(&(at - since));
        let repeated = line.is_some_and(|last| at - last.at < RIVEN_LINE_REPEAT);
        if !in_window || repeated {
            return None;
        }

        let generation = self.next_generation();
        self.chat_link = ChatLink::Open {
            since,
            line: Some(RivenLine { at, generation }),
        };
        Some(generation)
    }

    pub(super) fn chat_link_pending(&self, generation: u64) -> bool {
        matches!(
            self.chat_link,
            ChatLink::Open {
                line: Some(RivenLine { generation: open, .. }),
                ..
            } if open == generation
        )
    }
}

#[derive(Debug, Default)]
pub(super) struct Game {
    pub(super) mission_node: Option<String>,
    pub(super) mission_tier: Option<RelicTier>,
    pub(super) star_chart_open: bool,
    console: Option<f64>,
    hud_layers: u32,
}

impl Game {
    pub(super) fn set_mission(&mut self, node: Option<&str>, tier: Option<RelicTier>) {
        self.mission_node = node.map(str::to_owned);
        self.mission_tier = tier;
    }

    pub(super) fn requiem_mission(&self) -> bool {
        self.mission_tier == Some(RelicTier::Requiem)
    }

    pub(super) fn console_opened(&mut self, at: f64) {
        self.console = Some(at);
    }

    fn console_just_opened(&self, at: f64) -> bool {
        self.console
            .is_some_and(|opened| at - opened <= RELIC_SELECT_FROM_CONSOLE)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) enum Reward {
    #[default]
    Closed,
    Open {
        at: f64,
    },
    Dismissed {
        at: f64,
    },
}

impl Reward {
    pub(super) fn tiles_shown(&mut self, at: f64) {
        *self = Self::Open { at };
    }

    pub(super) fn input_reset(&mut self, at: f64) -> bool {
        match *self {
            Self::Open { at: shown } if at > shown => {
                *self = Self::Dismissed { at: shown };
                true
            }
            _ => false,
        }
    }

    pub(super) fn is_open(self) -> bool {
        matches!(self, Self::Open { .. })
    }

    fn just_shown(self, at: f64) -> bool {
        match self {
            Self::Open { at: shown } | Self::Dismissed { at: shown } => {
                at - shown < ENDLESS_FISSURE_NEXT_RELIC
            }
            Self::Closed => false,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) enum Recommendation {
    #[default]
    Closed,
    Open {
        at: f64,
        after_relic: bool,
        generation: u64,
    },
}

impl Recommendation {
    pub(super) fn input_reset(&mut self, at: f64) -> bool {
        match *self {
            Self::Open {
                at: opened,
                after_relic,
                ..
            } if !loading_reset(at - opened, after_relic) => {
                *self = Self::Closed;
                true
            }
            _ => false,
        }
    }
}

fn loading_reset(elapsed: f64, after_relic: bool) -> bool {
    elapsed < RESETS_WHILE_LOADING || (after_relic && elapsed < RESETS_WHILE_LOADING_AFTER_REWARDS)
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) enum ChatLink {
    #[default]
    Closed,
    Open {
        since: f64,
        line: Option<RivenLine>,
    },
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RivenLine {
    at: f64,
    generation: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum PendingAnswer {
    Cycle { row: Box<RivenRow> },
    KeepRoll,
}

#[derive(Debug, Default)]
pub(super) enum Cycling {
    #[default]
    Closed,
    Open {
        since: f64,
        pending: Option<(PendingAnswer, f64)>,
        roll_chosen: bool,
    },
}

impl Cycling {
    pub(super) fn screen_loaded(&mut self, at: f64) {
        match self {
            Self::Open { since, .. } => *since = at,
            Self::Closed => {
                *self = Self::Open {
                    since: at,
                    pending: None,
                    roll_chosen: false,
                };
            }
        }
    }

    pub(super) fn scene_torn_down(&mut self, at: f64) -> bool {
        match *self {
            Self::Open { since, .. } if at > since => {
                *self = Self::Closed;
                true
            }
            _ => false,
        }
    }

    pub(super) fn is_open(&self) -> bool {
        matches!(self, Self::Open { .. })
    }

    pub(super) fn arm(&mut self, answer: PendingAnswer, at: f64) {
        if let Self::Open { pending, .. } = self {
            *pending = Some((answer, at));
        }
    }

    pub(super) fn reject_answer(&mut self) {
        if let Self::Open { pending, .. } = self {
            *pending = None;
        }
    }

    pub(super) fn confirmed_answer(&mut self, at: f64) -> Option<PendingAnswer> {
        let Self::Open { pending, .. } = self else {
            return None;
        };
        let (answer, armed) = pending.take()?;
        (at - armed < CYCLE_ANSWER_TIMEOUT).then_some(answer)
    }

    pub(super) fn awaiting_riven(&self) -> bool {
        matches!(
            self,
            Self::Open {
                roll_chosen: true,
                ..
            }
        )
    }

    pub(super) fn roll_chosen(&mut self) {
        if let Self::Open { roll_chosen, .. } = self {
            *roll_chosen = true;
        }
    }

    pub(super) fn riven_resolved(&mut self) -> bool {
        match self {
            Self::Open { roll_chosen, .. } => std::mem::take(roll_chosen),
            Self::Closed => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reward_closes_on_the_reset_after_the_tiles() {
        let mut reward = Reward::default();
        assert!(!reward.input_reset(3683.0));
        assert!(!reward.is_open());

        reward.tiles_shown(3683.5);
        assert!(reward.is_open());
        assert!(!reward.input_reset(3683.0));
        assert!(reward.is_open());
        assert!(reward.input_reset(3698.5));
        assert!(!reward.is_open());
        assert!(!reward.input_reset(3699.0));
    }

    #[test]
    fn reward_shows_only_while_the_screen_is_open() {
        let mut screens = Screens::default();
        assert!(!screens.reward_shows());

        screens.reward.tiles_shown(8471.388);
        assert!(screens.reward_shows());
        assert!(screens.reward.input_reset(8486.407));
        assert!(!screens.reward_shows());

        screens
            .game
            .set_mission(Some("SolNode1"), Some(RelicTier::Requiem));
        screens.reward.tiles_shown(8600.0);
        assert!(!screens.reward_shows());
    }

    #[test]
    fn later_tiles_move_the_open_time() {
        let mut reward = Reward::default();
        reward.tiles_shown(8471.388);
        reward.tiles_shown(12.5);
        assert!(reward.input_reset(13.0));
    }

    #[test]
    fn solo_run_prints_no_tiles() {
        let mut screens = Screens::default();
        assert!(!screens.reward.input_reset(7501.721));
        assert!(!screens.reward_shows());

        let generation = screens.relic_select_loaded(7519.0).unwrap();
        assert!(screens.recommendation_pending(generation));
        assert!(screens.recommendation.input_reset(7519.5));
    }

    #[test]
    fn after_relic_window() {
        let mut reward = Reward::default();
        assert!(!reward.just_shown(1000.0));

        reward.tiles_shown(1000.0);
        assert!(reward.just_shown(1000.0));
        assert!(reward.just_shown(1022.9));
        assert!(!reward.just_shown(1023.0));
        assert!(reward.input_reset(1010.0));
        assert!(reward.just_shown(1022.9));
    }

    #[test]
    fn console_suppression_window() {
        let mut screens = Screens::default();
        assert!(screens.relic_select_loaded(100.0).is_some());

        screens.game.console_opened(100.0);
        assert!(screens.relic_select_loaded(100.0).is_none());
        assert!(screens.relic_select_loaded(101.0).is_none());
        assert!(screens.relic_select_loaded(101.001).is_some());
    }

    #[test]
    fn recommendation_self_close() {
        let mut screens = Screens::default();
        let generation = screens.relic_select_loaded(200.0).unwrap();

        assert!(screens.recommendation_pending(generation));
        assert!(!screens.recommendation.input_reset(200.0));
        assert!(!screens.recommendation.input_reset(200.4));
        assert!(screens.recommendation.input_reset(200.5));
        assert!(!screens.recommendation_pending(generation));
        assert!(!screens.recommendation.input_reset(260.0));

        screens.reward.tiles_shown(199.0);
        screens.relic_select_loaded(200.0);
        assert!(!screens.recommendation.input_reset(200.5));
        assert!(!screens.recommendation.input_reset(203.4));
        assert!(screens.recommendation.input_reset(203.5));
    }

    #[test]
    fn reopened_relic_select_outdates_the_first_scan() {
        let mut screens = Screens::default();
        let first = screens.relic_select_loaded(200.0).unwrap();
        let second = screens.relic_select_loaded(200.2).unwrap();

        assert!(!screens.recommendation_pending(first));
        assert!(screens.recommendation_pending(second));
    }

    #[test]
    fn riven_shows_once_per_dialog() {
        let mut screens = Screens::default();
        assert_eq!(screens.riven_line(4310.355), None);

        assert!(!screens.chat_link_layers(1, 4309.896));
        let first = screens.riven_line(4310.355).unwrap();

        assert!(screens.chat_link_pending(first));
        assert_eq!(screens.riven_line(4310.5), None);
        assert!(screens.chat_link_pending(first));
        let second = screens.riven_line(4310.8).unwrap();

        assert!(!screens.chat_link_pending(first));
        assert!(screens.chat_link_pending(second));
        assert_eq!(screens.riven_line(4311.9), None);
    }

    #[test]
    fn late_dialog_line() {
        let mut screens = Screens::default();
        assert!(!screens.chat_link_layers(1, 38684.285));
        let first = screens.riven_line(38684.598).unwrap();

        assert!(screens.chat_link_layers(0, 38687.449));
        assert!(!screens.chat_link_pending(first));
        assert_eq!(screens.riven_line(38684.598), None);

        assert!(!screens.chat_link_layers(1, 38688.289));
        assert_eq!(screens.riven_line(38684.598), None);
        assert!(screens.riven_line(38688.48).is_some());

        assert!(screens.chat_link_layers(0, 38690.582));
        assert_eq!(screens.riven_line(38690.9), None);
    }

    #[test]
    fn dialog_over_menu() {
        let mut screens = Screens::default();
        assert!(!screens.chat_link_layers(2, 26962.0));
        assert!(screens.riven_line(26962.395).is_some());
        assert!(screens.chat_link_layers(1, 26980.58));
        assert_eq!(screens.riven_line(26980.7), None);

        assert!(!screens.chat_link_layers(1, 38259.086));
        assert!(screens.riven_line(38259.395).is_some());
        assert!(screens.chat_link_layers(0, 38267.215));
        assert_eq!(screens.riven_line(38267.3), None);
    }

    #[test]
    fn confirmed_answer_expires() {
        let armed = |at| {
            let mut cycling = Cycling::default();
            cycling.screen_loaded(90.0);
            cycling.arm(PendingAnswer::KeepRoll, 96.931);
            cycling.confirmed_answer(at)
        };

        assert_eq!(armed(98.047), Some(PendingAnswer::KeepRoll));
        assert_eq!(armed(396.9), Some(PendingAnswer::KeepRoll));
        assert_eq!(armed(396.931), None);
        assert_eq!(armed(1000.0), None);

        let mut closed = Cycling::default();
        closed.arm(PendingAnswer::KeepRoll, 96.931);
        assert_eq!(closed.confirmed_answer(98.047), None);
    }

    #[test]
    fn cycling_closes_after_open_only() {
        let mut cycling = Cycling::default();
        assert!(!cycling.scene_torn_down(51731.293));

        cycling.screen_loaded(51731.441);
        assert!(!cycling.scene_torn_down(51731.277));
        assert!(cycling.is_open());
        assert!(cycling.scene_torn_down(51790.0));
        assert!(!cycling.scene_torn_down(51791.0));
    }

    #[test]
    fn reloaded_cycling_keeps_its_choice() {
        let mut cycling = Cycling::default();
        cycling.roll_chosen();
        assert!(!cycling.awaiting_riven());

        cycling.screen_loaded(10.0);
        cycling.roll_chosen();
        cycling.arm(PendingAnswer::KeepRoll, 11.0);
        cycling.screen_loaded(12.0);
        assert!(cycling.awaiting_riven());
        assert_eq!(
            cycling.confirmed_answer(12.5),
            Some(PendingAnswer::KeepRoll)
        );
        assert!(cycling.riven_resolved());
        assert!(!cycling.riven_resolved());

        cycling.roll_chosen();
        assert!(cycling.scene_torn_down(20.0));
        assert!(!cycling.awaiting_riven());
        assert!(!cycling.riven_resolved());
    }
}
