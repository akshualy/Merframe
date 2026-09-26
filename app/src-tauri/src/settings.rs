use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::{Store, StoreExt};
use wf_core::{AlertSettings, MasteryOptions, MasteryOrdering};
use wf_market::{Reach, TraderStatus};

pub const STORE_FILE: &str = "merframe.json";
const TOKEN_FILE: &str = "market_token";
const SETTINGS_KEY: &str = "settings";
const LEGACY_TOKEN_KEY: &str = "market_token";
const ACCOUNT_KEY: &str = "market_account";

pub const DISCORD_TEMPLATE: &str = "{tenno} started a conversation with you in Warframe.";

pub const OVERLAY_OPACITY_DEFAULT: u8 = 100;
pub const RECOMMENDATION_COUNT_DEFAULT: u8 = 6;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayMode {
    #[default]
    Auto,
    Windows,
    Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayPlacement {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Centre,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationRefinement {
    #[default]
    Radiant,
    Owned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub alerts: AlertSettings,
    #[serde(flatten)]
    pub notifications: NotificationSettings,
    #[serde(flatten)]
    pub discord: DiscordSettings,
    #[serde(flatten)]
    pub market: MarketSettings,
    pub world_state_interval_minutes: u32,
    #[serde(flatten)]
    pub inventory: InventorySettings,
    #[serde(flatten)]
    pub overlays: OverlaySettings,
    pub copy_relic_rewards: bool,
    pub check_for_updates: bool,
    pub force_log_file: bool,
    pub log_file_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationSettings {
    pub windows_notifications_enabled: bool,
    pub sound_notifications_enabled: bool,
    pub notification_only_background: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscordSettings {
    pub discord_notifications_enabled: bool,
    pub discord_webhook: Option<String>,
    pub discord_message_template: String,
    pub discord_fissure_alerts: bool,
    pub discord_timer_alerts: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketSettings {
    pub price_ttl_minutes: i64,
    pub market_poll_minutes: u32,
    pub market_auto_close: bool,
    pub take_rank_into_account: bool,
    pub market_trader_status: TraderStatus,
    pub market_trader_locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InventorySettings {
    pub include_founders_items: Option<bool>,
    #[serde(skip_serializing)]
    hide_founders_items: Option<bool>,
    pub include_forma_ranks: bool,
    pub show_full_inventory: bool,
    pub stats_tab_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlaySettings {
    pub overlays_enabled: bool,
    #[serde(flatten)]
    pub shown: OverlayToggles,
    #[serde(flatten)]
    pub placements: OverlayPlacements,
    pub overlay_recommendation_refinement: RecommendationRefinement,
    pub overlay_opacity: u8,
    pub overlay_recommendation_count: u8,
    pub overlay_mode: OverlayMode,
    pub overlay_only_while_game_active: bool,
    pub overlay_account_balance: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayToggles {
    pub overlay_relic_reward: bool,
    pub overlay_relic_recommendation: bool,
    pub overlay_riven: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayPlacements {
    pub overlay_relic_reward_placement: OverlayPlacement,
    pub overlay_relic_recommendation_placement: OverlayPlacement,
    pub overlay_riven_placement: OverlayPlacement,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            alerts: AlertSettings::default(),
            notifications: NotificationSettings::default(),
            discord: DiscordSettings::default(),
            market: MarketSettings::default(),
            world_state_interval_minutes: 5,
            inventory: InventorySettings::default(),
            overlays: OverlaySettings::default(),
            copy_relic_rewards: false,
            check_for_updates: true,
            force_log_file: false,
            log_file_path: None,
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            windows_notifications_enabled: true,
            sound_notifications_enabled: true,
            notification_only_background: true,
        }
    }
}

impl Default for DiscordSettings {
    fn default() -> Self {
        Self {
            discord_notifications_enabled: false,
            discord_webhook: None,
            discord_message_template: DISCORD_TEMPLATE.to_owned(),
            discord_fissure_alerts: false,
            discord_timer_alerts: false,
        }
    }
}

impl Default for MarketSettings {
    fn default() -> Self {
        Self {
            price_ttl_minutes: 15,
            market_poll_minutes: 5,
            market_auto_close: false,
            take_rank_into_account: true,
            market_trader_status: TraderStatus::Ingame,
            market_trader_locale: Some("en".to_owned()),
        }
    }
}

impl Default for InventorySettings {
    fn default() -> Self {
        Self {
            include_founders_items: None,
            hide_founders_items: None,
            include_forma_ranks: true,
            show_full_inventory: false,
            stats_tab_enabled: true,
        }
    }
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            overlays_enabled: true,
            shown: OverlayToggles::default(),
            placements: OverlayPlacements::default(),
            overlay_recommendation_refinement: RecommendationRefinement::Radiant,
            overlay_opacity: OVERLAY_OPACITY_DEFAULT,
            overlay_recommendation_count: RECOMMENDATION_COUNT_DEFAULT,
            overlay_mode: OverlayMode::Auto,
            overlay_only_while_game_active: true,
            overlay_account_balance: true,
        }
    }
}

impl Default for OverlayToggles {
    fn default() -> Self {
        Self {
            overlay_relic_reward: true,
            overlay_relic_recommendation: true,
            overlay_riven: true,
        }
    }
}

impl Default for OverlayPlacements {
    fn default() -> Self {
        Self {
            overlay_relic_reward_placement: OverlayPlacement::Centre,
            overlay_relic_recommendation_placement: OverlayPlacement::TopRight,
            overlay_riven_placement: OverlayPlacement::TopLeft,
        }
    }
}

impl Settings {
    pub fn log_path(&self) -> Option<PathBuf> {
        self.log_file_path.clone().or_else(wf_log::default_log_path)
    }

    pub fn trader_reach(&self) -> Reach {
        Reach {
            status: self.market.market_trader_status,
            locale: self.market.market_trader_locale.clone(),
        }
    }

    pub fn log_selection(&self) -> wf_log::Selection {
        if self.force_log_file {
            wf_log::Selection::File
        } else {
            wf_log::Selection::Auto
        }
    }

    pub fn world_state_interval(&self) -> Duration {
        let minutes = self.world_state_interval_minutes.clamp(5, 10);
        Duration::from_secs(u64::from(minutes) * 60)
    }

    pub fn price_interval(&self) -> Duration {
        let minutes = self.market.price_ttl_minutes.clamp(5, 120);
        Duration::from_secs(minutes.unsigned_abs() * 60)
    }

    pub fn market_poll_interval(&self) -> Duration {
        let minutes = self.market.market_poll_minutes.clamp(2, 60);
        Duration::from_secs(u64::from(minutes) * 60)
    }

    pub fn mastery_options(&self, ordering: MasteryOrdering) -> MasteryOptions {
        MasteryOptions {
            ordering,
            include_founders_items: self.inventory.founders_choice(),
            include_forma_ranks: self.inventory.include_forma_ranks,
        }
    }
}

impl InventorySettings {
    pub fn founders_choice(&self) -> Option<bool> {
        self.include_founders_items
            .or_else(|| self.hide_founders_items.map(|hidden| !hidden))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketAccount {
    pub ingame_name: String,
    pub slug: String,
    pub tier: String,
    pub mastery_rank: u32,
}

#[derive(Deserialize)]
struct StoredDocument {
    settings: Settings,
}

pub fn wants_overlay_windows(document: &[u8]) -> bool {
    let settings = serde_json::from_slice::<StoredDocument>(document)
        .map(|stored| stored.settings)
        .unwrap_or_default();
    settings.overlays.overlays_enabled && settings.overlays.overlay_mode != OverlayMode::Tab
}

fn store<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<Arc<Store<R>>> {
    app.store(crate::state::data_dir(app)?.join(STORE_FILE))
        .context("Opening the settings store")
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<Settings> {
    match store(app)?.get(SETTINGS_KEY) {
        Some(value) => serde_json::from_value(value).context("Stored settings are not readable"),
        None => Ok(Settings::default()),
    }
}

pub fn save<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> anyhow::Result<()> {
    let store = store(app)?;
    store.set(
        SETTINGS_KEY,
        serde_json::to_value(settings).context("Serializing the settings")?,
    );
    store.save().context("Writing the settings store")
}

fn token_path<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<PathBuf> {
    Ok(crate::state::data_dir(app)?.join(TOKEN_FILE))
}

pub fn token<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<Option<String>> {
    let path = token_path(app)?;
    match std::fs::read_to_string(&path) {
        Ok(token) => Ok(Some(token)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => moved_token(app, &path),
        Err(error) => Err(error).with_context(|| format!("Reading {}", path.display())),
    }
}

fn moved_token<R: Runtime>(app: &AppHandle<R>, path: &Path) -> anyhow::Result<Option<String>> {
    let store = store(app)?;
    let Some(token) = store
        .get(LEGACY_TOKEN_KEY)
        .and_then(|value| value.as_str().map(str::to_owned))
    else {
        return Ok(None);
    };
    write_token(path, &token)?;
    store.delete(LEGACY_TOKEN_KEY);
    store.save().context("Writing the settings store")?;
    tracing::info!("Session token moved to its own file");
    Ok(Some(token))
}

pub fn set_token<R: Runtime>(app: &AppHandle<R>, token: Option<&str>) -> anyhow::Result<()> {
    let path = token_path(app)?;
    match token {
        Some(token) => write_token(&path, token),
        None => match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).with_context(|| format!("Removing {}", path.display())),
        },
    }
}

fn write_token(path: &Path, token: &str) -> anyhow::Result<()> {
    std::fs::write(path, token).with_context(|| format!("Writing {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Restricting {}", path.display()))?;
    }
    Ok(())
}

pub fn account<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<Option<MarketAccount>> {
    match store(app)?.get(ACCOUNT_KEY) {
        Some(value) => serde_json::from_value(value)
            .map(Some)
            .context("Stored market account is not readable"),
        None => Ok(None),
    }
}

pub fn set_account<R: Runtime>(
    app: &AppHandle<R>,
    account: Option<&MarketAccount>,
) -> anyhow::Result<()> {
    let store = store(app)?;
    match account {
        Some(account) => store.set(
            ACCOUNT_KEY,
            serde_json::to_value(account).context("Serializing the market account")?,
        ),
        None => {
            store.delete(ACCOUNT_KEY);
        }
    }
    store.save().context("Writing the settings store")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use wf_core::TimerAlerts;

    use super::*;

    #[test]
    fn shipped_defaults() {
        let settings = Settings::default();
        assert!(settings.notifications.windows_notifications_enabled);
        assert!(settings.notifications.sound_notifications_enabled);
        assert!(!settings.discord.discord_notifications_enabled);
        assert_eq!(settings.discord.discord_webhook, None);
        assert_eq!(settings.discord.discord_message_template, DISCORD_TEMPLATE);
        assert!(settings.notifications.notification_only_background);
        assert_eq!(settings.market.price_ttl_minutes, 15);
        assert_eq!(settings.world_state_interval_minutes, 5);
        assert_eq!(settings.market.market_poll_minutes, 5);
        assert!(!settings.market.market_auto_close);
        assert_eq!(settings.inventory.include_founders_items, None);
        assert!(settings.inventory.include_forma_ranks);
        assert!(!settings.inventory.show_full_inventory);
        assert!(settings.inventory.stats_tab_enabled);
        assert!(settings.overlays.overlays_enabled);
        assert!(settings.overlays.shown.overlay_relic_reward);
        assert!(settings.overlays.shown.overlay_relic_recommendation);
        assert!(settings.overlays.shown.overlay_riven);
        assert_eq!(
            settings.overlays.placements.overlay_relic_reward_placement,
            OverlayPlacement::Centre
        );
        assert_eq!(
            settings
                .overlays
                .placements
                .overlay_relic_recommendation_placement,
            OverlayPlacement::TopRight
        );
        assert_eq!(
            settings.overlays.placements.overlay_riven_placement,
            OverlayPlacement::TopLeft
        );
        assert_eq!(
            settings.overlays.overlay_recommendation_refinement,
            RecommendationRefinement::Radiant
        );
        assert_eq!(settings.overlays.overlay_opacity, 100);
        assert_eq!(settings.overlays.overlay_recommendation_count, 6);
        assert_eq!(settings.overlays.overlay_mode, OverlayMode::Auto);
        assert!(settings.overlays.overlay_only_while_game_active);
        assert!(settings.overlays.overlay_account_balance);
        assert!(!settings.copy_relic_rewards);
        assert!(settings.check_for_updates);
        assert!(!settings.discord.discord_fissure_alerts);
        assert!(!settings.discord.discord_timer_alerts);
        assert_eq!(settings.market.market_trader_status, TraderStatus::Ingame);
        assert_eq!(settings.market.market_trader_locale.as_deref(), Some("en"));
        assert_eq!(settings.log_file_path, None);
        assert_eq!(
            settings.trader_reach(),
            Reach {
                status: TraderStatus::Ingame,
                locale: Some("en".to_owned()),
            }
        );
        assert!(!settings.alerts.fissure_notifications_enabled);
        assert!(settings.alerts.fissure_filters.is_empty());
        assert_eq!(settings.alerts.timers, TimerAlerts::default());
        assert_eq!(settings.alerts.timer_lead_secs, 180);
    }

    #[test]
    fn empty_document_default() {
        let stored: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(stored, Settings::default());
    }

    #[test]
    fn flat_document_round_trip() {
        let stored: Settings = serde_json::from_str(
            r#"{"alerts":{"fissure_notifications_enabled":true,"fissure_filters":[],"timers":[],"timer_lead_secs":120},
                "windows_notifications_enabled":false,"sound_notifications_enabled":true,
                "discord_notifications_enabled":true,"discord_webhook":"https://discord.example/hook",
                "discord_message_template":"{tenno} says hi",
                "notification_only_background":false,"price_ttl_minutes":30,
                "world_state_interval_minutes":7,"market_poll_minutes":10,"market_auto_close":true,
                "take_rank_into_account":false,"include_founders_items":false,"include_forma_ranks":false,
                "show_full_inventory":true,"stats_tab_enabled":false,"overlays_enabled":true,
                "overlay_relic_reward":false,"overlay_relic_recommendation":true,"overlay_riven":false,
                "overlay_relic_reward_placement":"bottom_left",
                "overlay_relic_recommendation_placement":"centre","overlay_riven_placement":"bottom_right",
                "overlay_recommendation_refinement":"owned","overlay_opacity":55,
                "overlay_recommendation_count":9,"overlay_mode":"windows",
                "overlay_only_while_game_active":false,"force_log_file":true}"#,
        )
        .unwrap();
        assert!(stored.alerts.fissure_notifications_enabled);
        assert_eq!(stored.alerts.timer_lead_secs, 120);
        assert!(!stored.notifications.windows_notifications_enabled);
        assert!(stored.notifications.sound_notifications_enabled);
        assert!(!stored.notifications.notification_only_background);
        assert!(stored.discord.discord_notifications_enabled);
        assert_eq!(
            stored.discord.discord_webhook.as_deref(),
            Some("https://discord.example/hook")
        );
        assert_eq!(stored.discord.discord_message_template, "{tenno} says hi");
        assert_eq!(stored.market.price_ttl_minutes, 30);
        assert_eq!(stored.market.market_poll_minutes, 10);
        assert!(stored.market.market_auto_close);
        assert!(!stored.market.take_rank_into_account);
        assert_eq!(stored.world_state_interval_minutes, 7);
        assert_eq!(stored.inventory.include_founders_items, Some(false));
        assert!(!stored.inventory.include_forma_ranks);
        assert!(stored.inventory.show_full_inventory);
        assert!(!stored.inventory.stats_tab_enabled);
        assert!(stored.overlays.overlays_enabled);
        assert!(!stored.overlays.shown.overlay_relic_reward);
        assert!(stored.overlays.shown.overlay_relic_recommendation);
        assert!(!stored.overlays.shown.overlay_riven);
        assert_eq!(
            stored.overlays.placements.overlay_relic_reward_placement,
            OverlayPlacement::BottomLeft
        );
        assert_eq!(
            stored
                .overlays
                .placements
                .overlay_relic_recommendation_placement,
            OverlayPlacement::Centre
        );
        assert_eq!(
            stored.overlays.placements.overlay_riven_placement,
            OverlayPlacement::BottomRight
        );
        assert_eq!(
            stored.overlays.overlay_recommendation_refinement,
            RecommendationRefinement::Owned
        );
        assert_eq!(stored.overlays.overlay_opacity, 55);
        assert_eq!(stored.overlays.overlay_recommendation_count, 9);
        assert_eq!(stored.overlays.overlay_mode, OverlayMode::Windows);
        assert!(!stored.overlays.overlay_only_while_game_active);
        assert!(stored.force_log_file);

        let saved: serde_json::Value = serde_json::to_value(&stored).unwrap();
        let keys: Vec<&str> = saved
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert!(keys.iter().all(|key| !key.contains('.')));
        assert!(saved.get("notifications").is_none());
        assert!(saved.get("overlays").is_none());
        assert_eq!(saved["overlay_riven"], false);
        assert_eq!(saved["price_ttl_minutes"], 30);
    }

    #[test]
    fn flipped_defaults_stay_off() {
        let stored: Settings = serde_json::from_str(
            r#"{"windows_notifications_enabled":false,"sound_notifications_enabled":false,
                "notification_only_background":false,"stats_tab_enabled":false,
                "overlays_enabled":false,"overlay_relic_reward":false,
                "overlay_relic_recommendation":false,"overlay_riven":false}"#,
        )
        .unwrap();
        assert!(!stored.notifications.windows_notifications_enabled);
        assert!(!stored.notifications.sound_notifications_enabled);
        assert!(!stored.notifications.notification_only_background);
        assert!(!stored.inventory.stats_tab_enabled);
        assert!(!stored.overlays.overlays_enabled);
        assert!(!stored.overlays.shown.overlay_relic_reward);
        assert!(!stored.overlays.shown.overlay_relic_recommendation);
        assert!(!stored.overlays.shown.overlay_riven);
    }

    #[test]
    fn pre_overlay_document() {
        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert!(stored.overlays.overlays_enabled);
        assert!(stored.overlays.shown.overlay_relic_reward);
        assert!(stored.overlays.shown.overlay_relic_recommendation);
        assert!(stored.overlays.shown.overlay_riven);
        assert_eq!(stored.overlays.overlay_mode, OverlayMode::Auto);
    }

    #[test]
    fn pre_placement_document() {
        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert_eq!(
            stored.overlays.placements.overlay_relic_reward_placement,
            OverlayPlacement::Centre
        );
        assert_eq!(
            stored
                .overlays
                .placements
                .overlay_relic_recommendation_placement,
            OverlayPlacement::TopRight
        );
        assert_eq!(
            stored.overlays.placements.overlay_riven_placement,
            OverlayPlacement::TopLeft
        );
        assert_eq!(
            stored.overlays.overlay_recommendation_refinement,
            RecommendationRefinement::Radiant
        );
    }

    #[test]
    fn placements_round_trip() {
        let stored: Settings = serde_json::from_str(
            r#"{"overlay_relic_reward_placement":"bottom_left",
                "overlay_relic_recommendation_placement":"bottom_right",
                "overlay_riven_placement":"centre",
                "overlay_recommendation_refinement":"owned",
                "overlay_opacity":70,
                "overlay_recommendation_count":9}"#,
        )
        .unwrap();
        assert_eq!(
            stored.overlays.placements.overlay_relic_reward_placement,
            OverlayPlacement::BottomLeft
        );
        assert_eq!(
            stored
                .overlays
                .placements
                .overlay_relic_recommendation_placement,
            OverlayPlacement::BottomRight
        );
        assert_eq!(
            stored.overlays.placements.overlay_riven_placement,
            OverlayPlacement::Centre
        );
        assert_eq!(
            stored.overlays.overlay_recommendation_refinement,
            RecommendationRefinement::Owned
        );
        assert_eq!(stored.overlays.overlay_opacity, 70);
        assert_eq!(stored.overlays.overlay_recommendation_count, 9);

        let saved = serde_json::to_string(&stored).unwrap();
        assert!(saved.contains(r#""overlay_relic_reward_placement":"bottom_left""#));
        assert!(saved.contains(r#""overlay_riven_placement":"centre""#));
        assert!(saved.contains(r#""overlay_recommendation_refinement":"owned""#));

        let top: Settings =
            serde_json::from_str(r#"{"overlay_riven_placement":"top_right"}"#).unwrap();
        assert_eq!(
            top.overlays.placements.overlay_riven_placement,
            OverlayPlacement::TopRight
        );
    }

    #[test]
    fn only_while_game_active_default() {
        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert!(stored.overlays.overlay_only_while_game_active);

        let off: Settings =
            serde_json::from_str(r#"{"overlay_only_while_game_active":false}"#).unwrap();
        assert!(!off.overlays.overlay_only_while_game_active);

        let saved = serde_json::to_string(&off).unwrap();
        assert!(saved.contains(r#""overlay_only_while_game_active":false"#));
    }

    #[test]
    fn overlay_mode_round_trip() {
        let stored: Settings = serde_json::from_str(
            r#"{"overlay_riven":false,"overlay_relic_reward":false,"overlay_mode":"tab"}"#,
        )
        .unwrap();
        assert!(!stored.overlays.shown.overlay_riven);
        assert!(!stored.overlays.shown.overlay_relic_reward);
        assert!(stored.overlays.shown.overlay_relic_recommendation);
        assert_eq!(stored.overlays.overlay_mode, OverlayMode::Tab);

        let saved = serde_json::to_string(&stored).unwrap();
        assert!(saved.contains(r#""overlay_mode":"tab""#));

        let windows: Settings = serde_json::from_str(r#"{"overlay_mode":"windows"}"#).unwrap();
        assert_eq!(windows.overlays.overlay_mode, OverlayMode::Windows);
    }

    #[test]
    fn log_selection_default() {
        let settings = Settings::default();
        assert!(!settings.force_log_file);
        assert_eq!(settings.log_selection(), wf_log::Selection::Auto);

        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert_eq!(stored.log_selection(), wf_log::Selection::Auto);

        let forced: Settings = serde_json::from_str(r#"{"force_log_file":true}"#).unwrap();
        assert_eq!(forced.log_selection(), wf_log::Selection::File);
    }

    #[test]
    fn discord_opt_in() {
        let settings = Settings::default();
        assert!(!settings.discord.discord_notifications_enabled);
        assert_eq!(settings.discord.discord_webhook, None);
        assert!(
            settings
                .discord
                .discord_message_template
                .contains("{tenno}")
        );

        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert_eq!(stored.discord.discord_message_template, DISCORD_TEMPLATE);

        let chosen: Settings =
            serde_json::from_str(r#"{"discord_message_template":"{tenno}?"}"#).unwrap();
        assert_eq!(chosen.discord.discord_message_template, "{tenno}?");
    }

    #[test]
    fn auto_close_opt_in() {
        assert!(!Settings::default().market.market_auto_close);
        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert!(!stored.market.market_auto_close);
        let enabled: Settings = serde_json::from_str(r#"{"market_auto_close":true}"#).unwrap();
        assert!(enabled.market.market_auto_close);
    }

    #[test]
    fn partial_document() {
        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":30}"#).unwrap();
        assert_eq!(stored.market.price_ttl_minutes, 30);
        assert!(stored.notifications.windows_notifications_enabled);
        assert!(stored.inventory.stats_tab_enabled);
        assert_eq!(stored.discord.discord_message_template, DISCORD_TEMPLATE);
    }

    #[test]
    fn show_full_inventory_default() {
        assert!(!Settings::default().inventory.show_full_inventory);
        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert!(!stored.inventory.show_full_inventory);
        let asked: Settings = serde_json::from_str(r#"{"show_full_inventory":true}"#).unwrap();
        assert!(asked.inventory.show_full_inventory);
    }

    #[test]
    fn price_ttl_default() {
        let settings = Settings::default();
        assert_eq!(settings.market.price_ttl_minutes, 15);
        assert_eq!(settings.price_interval().as_secs(), 900);

        let stored: Settings = serde_json::from_str(r#"{"market_poll_minutes":5}"#).unwrap();
        assert_eq!(stored.market.price_ttl_minutes, 15);
    }

    #[test]
    fn price_interval_clamp() {
        let interval = |minutes: i64| {
            Settings {
                market: MarketSettings {
                    price_ttl_minutes: minutes,
                    ..MarketSettings::default()
                },
                ..Settings::default()
            }
            .price_interval()
        };
        assert_eq!(interval(-30), Duration::from_secs(300));
        assert_eq!(interval(0), Duration::from_secs(300));
        assert_eq!(interval(4), Duration::from_secs(300));
        assert_eq!(interval(5), Duration::from_secs(300));
        assert_eq!(interval(60).as_secs(), 3600);
        assert_eq!(interval(120).as_secs(), 7200);
        assert_eq!(interval(600).as_secs(), 7200);
    }

    #[test]
    fn world_state_interval_default() {
        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert_eq!(stored.world_state_interval_minutes, 5);
        assert_eq!(stored.world_state_interval(), Duration::from_secs(300));
    }

    #[test]
    fn world_state_interval_clamp() {
        let interval = |minutes: u32| {
            Settings {
                world_state_interval_minutes: minutes,
                ..Settings::default()
            }
            .world_state_interval()
        };
        assert_eq!(interval(0), Duration::from_secs(300));
        assert_eq!(interval(4), Duration::from_secs(300));
        assert_eq!(interval(5), Duration::from_secs(300));
        assert_eq!(interval(7), Duration::from_secs(420));
        assert_eq!(interval(10), Duration::from_secs(600));
        assert_eq!(interval(600), Duration::from_secs(600));
    }

    #[test]
    fn market_poll_clamp() {
        let settings = Settings::default();
        assert_eq!(settings.market.market_poll_minutes, 5);
        assert_eq!(settings.market_poll_interval(), Duration::from_secs(300));

        let interval = |minutes: u32| {
            Settings {
                market: MarketSettings {
                    market_poll_minutes: minutes,
                    ..MarketSettings::default()
                },
                ..Settings::default()
            }
            .market_poll_interval()
        };
        assert_eq!(interval(0), Duration::from_secs(120));
        assert_eq!(interval(1), Duration::from_secs(120));
        assert_eq!(interval(2), Duration::from_secs(120));
        assert_eq!(interval(15).as_secs(), 900);
        assert_eq!(interval(600).as_secs(), 3600);

        let stored: Settings = serde_json::from_str(r#"{"price_ttl_minutes":15}"#).unwrap();
        assert_eq!(stored.market.market_poll_minutes, 5);
    }

    #[test]
    fn founders_choice_default() {
        let settings = Settings::default();
        assert_eq!(settings.inventory.founders_choice(), None);
        assert!(settings.inventory.include_forma_ranks);

        let options = settings.mastery_options(wf_core::MasteryOrdering::FromRelics);
        assert_eq!(options.ordering, wf_core::MasteryOrdering::FromRelics);
        assert_eq!(options.include_founders_items, None);
        assert!(options.include_forma_ranks);
        assert_eq!(
            options,
            wf_core::MasteryOptions {
                ordering: wf_core::MasteryOrdering::FromRelics,
                ..wf_core::MasteryOptions::default()
            }
        );

        let stored: Settings =
            serde_json::from_str(r#"{"include_founders_items":true,"include_forma_ranks":false}"#)
                .unwrap();
        let options = stored.mastery_options(wf_core::MasteryOrdering::Closest);
        assert_eq!(options.include_founders_items, Some(true));
        assert!(!options.include_forma_ranks);
    }

    #[test]
    fn legacy_hide_founders_flag() {
        let stored: Settings = serde_json::from_str(r#"{"hide_founders_items":true}"#).unwrap();
        assert_eq!(stored.inventory.founders_choice(), Some(false));
        assert_eq!(stored.inventory.include_founders_items, None);

        let shown: Settings = serde_json::from_str(r#"{"hide_founders_items":false}"#).unwrap();
        assert_eq!(shown.inventory.founders_choice(), Some(true));

        let chosen: Settings =
            serde_json::from_str(r#"{"hide_founders_items":true,"include_founders_items":true}"#)
                .unwrap();
        assert_eq!(chosen.inventory.founders_choice(), Some(true));

        let saved = serde_json::to_string(&stored).unwrap();
        assert!(!saved.contains("hide_founders_items"));
    }

    #[test]
    fn wants_overlay_windows_from_bytes() {
        let shipped = serde_json::json!({ "settings": Settings::default() }).to_string();
        assert!(wants_overlay_windows(shipped.as_bytes()));
        assert!(wants_overlay_windows(
            br#"{"settings":{"overlay_mode":"windows"}}"#
        ));
        assert!(!wants_overlay_windows(
            br#"{"settings":{"overlays_enabled":false}}"#
        ));
        assert!(!wants_overlay_windows(
            br#"{"settings":{"overlay_mode":"tab"}}"#
        ));
        assert!(wants_overlay_windows(b""));
        assert!(wants_overlay_windows(b"{\"settings\":"));
    }
}
