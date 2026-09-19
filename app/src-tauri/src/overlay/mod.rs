use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tracing::{debug, info, warn};
use wf_core::{CoreEvent, RivenRow};
use wf_log::Event as LogEvent;
use wf_worldstate::RelicTier;

use crate::runtime::emit;
use crate::settings::{
    OVERLAY_OPACITY_DEFAULT, OverlayPlacement, RecommendationRefinement, Settings,
};
use crate::state::{AppState, lock, read};

mod relic;
mod riven;
mod session;
mod window;

pub use riven::riven_item_type;
#[cfg(target_os = "linux")]
pub use session::adopt_xwayland;
#[cfg(target_os = "linux")]
pub use session::x_display_listening;
pub use session::{
    FORCE_ENV, OverlaySupport, Session, Startup, XWAYLAND_ENV, probe, session_of, support_for,
    uses_windows, x_display_number,
};

use relic::{
    recommendation_after_relic, recommendation_stays, recommendation_suppressed, requiem_mission,
    reward_screen_closes, scan_recommendation,
};
use riven::{
    on_dialog_answer, purchase_dialog_closes, riven_shows, show_linked_riven,
    show_station_selection, station_closes,
};
use window::{bounds_for, logical, open, park, retire, reveal, screen_of};

pub const KINDS: [Kind; 3] = [Kind::RelicReward, Kind::RelicRecommendation, Kind::Riven];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    RelicReward,
    RelicRecommendation,
    Riven,
}

impl Kind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::RelicReward => "overlay-relic",
            Self::RelicRecommendation => "overlay-recommend",
            Self::Riven => "overlay-riven",
        }
    }

    pub(super) const fn route(self) -> &'static str {
        match self {
            Self::RelicReward => "index.html#/overlay/relic",
            Self::RelicRecommendation => "index.html#/overlay/recommend",
            Self::Riven => "index.html#/overlay/riven",
        }
    }

    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::RelicReward => "Merframe relic rewards",
            Self::RelicRecommendation => "Merframe relic recommendation",
            Self::Riven => "Merframe riven",
        }
    }
}

pub fn enabled(settings: &Settings, kind: Kind) -> bool {
    settings.overlays.overlays_enabled
        && match kind {
            Kind::RelicReward => settings.overlays.shown.overlay_relic_reward,
            Kind::RelicRecommendation => settings.overlays.shown.overlay_relic_recommendation,
            Kind::Riven => settings.overlays.shown.overlay_riven,
        }
}

fn window_shows(only_while_game_active: bool, focused: Option<bool>, due: bool) -> bool {
    due && !(only_while_game_active && focused == Some(false))
}

fn placement_of(settings: &Settings, kind: Kind) -> OverlayPlacement {
    match kind {
        Kind::RelicReward => settings.overlays.placements.overlay_relic_reward_placement,
        Kind::RelicRecommendation => {
            settings
                .overlays
                .placements
                .overlay_relic_recommendation_placement
        }
        Kind::Riven => settings.overlays.placements.overlay_riven_placement,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RewardTrigger {
    pub relic: Option<String>,
    pub rewards: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecommendationTrigger {
    pub tier: Option<&'static str>,
    pub refinement: RecommendationRefinement,
    pub count: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RivenTrigger {
    pub item_type: String,
    pub before: Option<RivenRow>,
    pub linked: Option<RivenRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OverlayState {
    pub seq: u64,
    pub opacity: u8,
    pub reward: Option<RewardTrigger>,
    pub recommendation: Option<RecommendationTrigger>,
    pub riven: Option<RivenTrigger>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            seq: 0,
            opacity: OVERLAY_OPACITY_DEFAULT,
            reward: None,
            recommendation: None,
            riven: None,
        }
    }
}

impl OverlayState {
    fn occupied(&self, kind: Kind) -> bool {
        match kind {
            Kind::RelicReward => self.reward.is_some(),
            Kind::RelicRecommendation => self.recommendation.is_some(),
            Kind::Riven => self.riven.is_some(),
        }
    }

    fn clear(&mut self, kind: Kind) {
        match kind {
            Kind::RelicReward => self.reward = None,
            Kind::RelicRecommendation => self.recommendation = None,
            Kind::Riven => self.riven = None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingAnswer {
    Cycle { riven: String },
    KeepRoll,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Marks {
    relic_rewards: Option<f64>,
    recommendation: Option<f64>,
    console: Option<f64>,
    after_relic: bool,
    purchase_dialog: Option<f64>,
    riven: Option<f64>,
    star_chart: Option<f64>,
    star_chart_hidden: Option<f64>,
    mission_tier: Option<RelicTier>,
    hud_visibility: u32,
    reroll_screen: Option<f64>,
    pending: Option<(PendingAnswer, f64)>,
    choice_made: bool,
}

fn since(mark: Option<f64>, at: f64) -> Option<f64> {
    mark.map(|when| at - when)
}

#[derive(Default)]
pub struct Overlays {
    relic_picker: Mutex<wf_scan::RelicPicker>,
    lua: Mutex<Option<wf_scan::LuaState>>,
    state: Mutex<OverlayState>,
    recommendation_ticket: AtomicU64,
    riven_ticket: AtomicU64,
    marks: Mutex<Marks>,
    windows: Mutex<HashMap<&'static str, WindowLife>>,
}

const IDLE_LIFETIME: Duration = Duration::from_secs(300);

struct WindowLife {
    requested: Instant,
    ready: bool,
    idle_since: Option<Instant>,
}

impl Overlays {
    pub fn snapshot(&self) -> OverlayState {
        lock(&self.state).clone()
    }
}

pub fn on_core_event<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, event: &CoreEvent) {
    if let CoreEvent::RelicRewardScreen { relic, rewards } = event {
        if requiem_mission(&lock(&state.overlays.marks)) {
            return;
        }
        let trigger = RewardTrigger {
            relic: relic.clone(),
            rewards: rewards.clone(),
        };
        show(app, state, Kind::RelicReward, |slots| {
            slots.reward = Some(trigger);
        });
    }
}

pub fn on_log_event<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    event: &LogEvent,
    at: f64,
) {
    match event {
        LogEvent::RelicRewardsShown => lock(&state.overlays.marks).relic_rewards = Some(at),
        LogEvent::ConsoleOpened => {
            lock(&state.overlays.marks).console = Some(at);
            preload(app, state, Kind::Riven);
        }
        LogEvent::StarChartOpened => {
            lock(&state.overlays.marks).star_chart = Some(at);
            preload(app, state, Kind::RelicRecommendation);
        }
        LogEvent::RelicRewardScreenOpened => {
            preload(app, state, Kind::RelicReward);
            preload(app, state, Kind::RelicRecommendation);
        }
        LogEvent::StarChartHidden => lock(&state.overlays.marks).star_chart_hidden = Some(at),
        LogEvent::MissionCleared => lock(&state.overlays.marks).mission_tier = None,
        LogEvent::MissionSet { void_tier, .. } => {
            lock(&state.overlays.marks).mission_tier =
                void_tier.as_deref().and_then(RelicTier::from_modifier);
        }
        LogEvent::RelicSelectScreenLoaded => {
            {
                let mut marks = lock(&state.overlays.marks);
                if recommendation_suppressed(&marks, at) {
                    return;
                }
                marks.after_relic = recommendation_after_relic(&marks, at);
                marks.recommendation = Some(at);
            }
            let ticket = state
                .overlays
                .recommendation_ticket
                .fetch_add(1, Ordering::Relaxed)
                + 1;
            scan_recommendation(app, state, ticket);
        }
        LogEvent::InputMappingReset => {
            if reward_screen_closes(&lock(&state.overlays.marks), at) {
                hide(app, state, Kind::RelicReward);
            }
            let stays = recommendation_stays(&lock(&state.overlays.marks), at);
            if !stays {
                state
                    .overlays
                    .recommendation_ticket
                    .fetch_add(1, Ordering::Relaxed);
                hide(app, state, Kind::RelicRecommendation);
            }
        }
        LogEvent::PurchaseDialogHudVisibility { count } => {
            let dropped = purchase_dialog_closes(&mut lock(&state.overlays.marks), *count, at);
            if dropped {
                state.overlays.riven_ticket.fetch_add(1, Ordering::Relaxed);
                hide(app, state, Kind::Riven);
            }
        }
        LogEvent::RivenDialog { item_path } => {
            let Some(item_type) = riven_item_type(item_path) else {
                return;
            };
            {
                let mut marks = lock(&state.overlays.marks);
                if !riven_shows(&marks, at) {
                    return;
                }
                marks.riven = Some(at);
            }
            let ticket = state.overlays.riven_ticket.fetch_add(1, Ordering::Relaxed) + 1;
            show_linked_riven(app, state, item_type, ticket);
        }
        LogEvent::RivenRerollScreenLoaded => {
            lock(&state.overlays.marks).reroll_screen = Some(at);
            show_station_selection(app, state);
        }
        LogEvent::RivenCycleDialog { riven, .. } => {
            debug!(riven, "Cycle confirmation dialog opened");
            let answer = PendingAnswer::Cycle {
                riven: riven.clone(),
            };
            lock(&state.overlays.marks).pending = Some((answer, at));
        }
        LogEvent::RivenCycleKeepDialog => {
            lock(&state.overlays.marks).pending = Some((PendingAnswer::KeepRoll, at));
        }
        LogEvent::WindowFocus { focused } => follow_game_window(app, state, *focused),
        LogEvent::DialogAccepted => on_dialog_answer(app, state, at),
        LogEvent::SceneTornDown => {
            let closed = station_closes(&mut lock(&state.overlays.marks), at);
            if closed {
                hide(app, state, Kind::Riven);
            }
        }
        _ => {}
    }
}

fn attached_game() -> Option<Box<dyn wf_mem::MemoryReader>> {
    match wf_mem::open_game() {
        Ok(reader) => Some(reader),
        Err(error) => {
            debug!(%error, "Game process memory is not readable");
            None
        }
    }
}

pub fn on_inventory_updated<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let rerolling = {
        let mut slots = lock(&state.overlays.state);
        let rerolling = slots
            .riven
            .as_ref()
            .is_some_and(|trigger| trigger.before.is_some());
        if rerolling {
            slots.seq += 1;
        }
        rerolling
    };
    if rerolling {
        broadcast(app, state);
    }
}

pub fn apply_from<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let (wanted, only_while_game_active) = {
        let settings = read(&state.settings);
        lock(&state.overlays.state).opacity = settings.overlays.overlay_opacity;
        let windowed = uses_windows(
            settings.overlays.overlay_mode,
            &read(&state.status).overlay_support,
        );
        let rows = settings.overlays.overlay_recommendation_count;
        (
            KINDS.map(|kind| {
                (
                    windowed && enabled(&settings, kind),
                    placement_of(&settings, kind),
                    rows,
                )
            }),
            settings.overlays.overlay_only_while_game_active,
        )
    };
    let screen = screen_of(app);
    for (kind, (keep, placement, rows)) in KINDS.into_iter().zip(wanted) {
        let bounds = logical(bounds_for(kind, placement, rows, screen), screen.scale);
        match (app.get_webview_window(kind.label()), keep) {
            (Some(window), false) => {
                lock(&state.overlays.windows).remove(kind.label());
                if let Err(error) = window.close() {
                    warn!(
                        label = kind.label(),
                        %error,
                        "Overlay window close failed",
                    );
                }
            }
            (Some(window), true) => {
                if let Err(error) = window
                    .set_size(tauri::LogicalSize::new(bounds.width, bounds.height))
                    .and_then(|()| {
                        window.set_position(tauri::LogicalPosition::new(bounds.x, bounds.y))
                    })
                    .and_then(|()| window.set_zoom(1.0 / screen.scale))
                {
                    warn!(
                        label = kind.label(),
                        %error,
                        "Overlay window resize failed"
                    );
                }
            }
            (None, _) => {}
        }
        if keep {
            let due = lock(&state.overlays.state).occupied(kind);
            if window_shows(only_while_game_active, state.focus.focused(), due) {
                surface(app, state, kind);
            } else {
                rest(app, state, kind);
            }
        }
    }
    broadcast(app, state);
}

fn show<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    kind: Kind,
    fill: impl FnOnce(&mut OverlayState),
) {
    let (windowed, only_while_game_active) = {
        let settings = read(&state.settings);
        if !enabled(&settings, kind) {
            return;
        }
        (
            uses_windows(
                settings.overlays.overlay_mode,
                &read(&state.status).overlay_support,
            ),
            settings.overlays.overlay_only_while_game_active,
        )
    };
    {
        let mut slots = lock(&state.overlays.state);
        slots.seq += 1;
        fill(&mut slots);
    }
    broadcast(app, state);
    if window_shows(only_while_game_active, state.focus.focused(), windowed) {
        surface(app, state, kind);
    }
}

fn load<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, kind: Kind) {
    if app.get_webview_window(kind.label()).is_some() {
        return;
    }
    let (placement, rows) = {
        let settings = read(&state.settings);
        (
            placement_of(&settings, kind),
            settings.overlays.overlay_recommendation_count,
        )
    };
    let screen = screen_of(app);
    let bounds = logical(bounds_for(kind, placement, rows, screen), screen.scale);
    lock(&state.overlays.windows).insert(
        kind.label(),
        WindowLife {
            requested: Instant::now(),
            ready: false,
            idle_since: None,
        },
    );
    open(app, kind, bounds, screen.scale);
}

fn surface<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, kind: Kind) {
    load(app, state, kind);
    let ready = lock(&state.overlays.windows)
        .get_mut(kind.label())
        .is_some_and(|life| {
            life.idle_since = None;
            life.ready
        });
    if ready {
        reveal(app, kind);
    }
}

fn rest<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, kind: Kind) {
    park(app, kind);
    let idle_since = Instant::now();
    match lock(&state.overlays.windows).get_mut(kind.label()) {
        Some(life) if life.idle_since.is_none() => life.idle_since = Some(idle_since),
        _ => return,
    }
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(IDLE_LIFETIME).await;
        let expired = {
            let mut windows = lock(&owned.overlays.windows);
            let expired = windows
                .get(kind.label())
                .is_some_and(|life| life.idle_since == Some(idle_since));
            if expired {
                windows.remove(kind.label());
            }
            expired
        };
        if expired {
            debug!(label = kind.label(), "Idle overlay window closed");
            retire(&app, kind);
        }
    });
}

fn preload<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, kind: Kind) {
    let windowed = {
        let settings = read(&state.settings);
        enabled(&settings, kind)
            && uses_windows(
                settings.overlays.overlay_mode,
                &read(&state.status).overlay_support,
            )
    };
    if !windowed || app.get_webview_window(kind.label()).is_some() {
        return;
    }
    load(app, state, kind);
    rest(app, state, kind);
}

pub fn on_page_ready<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, label: &str) {
    let Some(kind) = KINDS.into_iter().find(|kind| kind.label() == label) else {
        return;
    };
    let waited = {
        let mut windows = lock(&state.overlays.windows);
        let Some(life) = windows.get_mut(label) else {
            return;
        };
        life.ready = true;
        life.requested.elapsed()
    };
    info!(label, millis = waited.as_millis(), "Overlay page ready");
    let settings = read(&state.settings);
    let due = enabled(&settings, kind) && lock(&state.overlays.state).occupied(kind);
    if window_shows(
        settings.overlays.overlay_only_while_game_active,
        state.focus.focused(),
        due,
    ) {
        reveal(app, kind);
    }
}

pub fn on_content_shrank<R: Runtime>(window: &tauri::WebviewWindow<R>) {
    if !session::keeps_vacated_pixels() {
        return;
    }
    let label = window.label();
    let nudged = window.outer_size().and_then(|size| {
        let height = if size.height % 2 == 0 {
            size.height + 1
        } else {
            size.height - 1
        };
        window.set_size(tauri::PhysicalSize::new(size.width, height))
    });
    match nudged {
        Ok(()) => debug!(label, "Overlay window resized after its content shrank"),
        Err(error) => warn!(label, %error, "Overlay window resize failed"),
    }
}

fn hide<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, kind: Kind) {
    {
        let mut slots = lock(&state.overlays.state);
        if !slots.occupied(kind) {
            return;
        }
        slots.seq += 1;
        slots.clear(kind);
    }
    broadcast(app, state);
    rest(app, state, kind);
}

fn follow_game_window<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, focused: bool) {
    let settings = read(&state.settings);
    if !uses_windows(
        settings.overlays.overlay_mode,
        &read(&state.status).overlay_support,
    ) {
        return;
    }
    for kind in KINDS {
        let due = enabled(&settings, kind) && lock(&state.overlays.state).occupied(kind);
        if window_shows(
            settings.overlays.overlay_only_while_game_active,
            Some(focused),
            due,
        ) {
            surface(app, state, kind);
        } else {
            rest(app, state, kind);
        }
    }
}

fn broadcast<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    emit(app, "overlay-state", state.overlays.snapshot());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn enabled_needs_both_switches() {
        let settings = Settings::default();
        assert!(enabled(&settings, Kind::RelicReward));
        assert!(enabled(&settings, Kind::RelicRecommendation));
        assert!(enabled(&settings, Kind::Riven));

        let master_off: Settings = serde_json::from_str(r#"{"overlays_enabled":false}"#).unwrap();
        assert!(!enabled(&master_off, Kind::RelicReward));
        assert!(!enabled(&master_off, Kind::RelicRecommendation));
        assert!(!enabled(&master_off, Kind::Riven));

        let one_toggle_off: Settings = serde_json::from_str(r#"{"overlay_riven":false}"#).unwrap();
        assert!(!enabled(&one_toggle_off, Kind::Riven));
        assert!(enabled(&one_toggle_off, Kind::RelicReward));
    }

    #[test]
    fn background_hide_setting() {
        for due in [false, true] {
            assert_eq!(window_shows(true, None, due), due);
            assert_eq!(window_shows(true, Some(true), due), due);
            assert!(!window_shows(true, Some(false), due));

            assert_eq!(window_shows(false, None, due), due);
            assert_eq!(window_shows(false, Some(true), due), due);
            assert_eq!(window_shows(false, Some(false), due), due);
        }
    }

    #[test]
    fn kind_label_and_route() {
        let labels: Vec<&str> = super::KINDS.iter().map(|kind| kind.label()).collect();
        assert_eq!(
            labels,
            vec!["overlay-relic", "overlay-recommend", "overlay-riven"]
        );
        for kind in super::KINDS {
            assert!(kind.route().starts_with("index.html#/overlay/"));
        }
    }
}
