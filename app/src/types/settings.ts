export type OverlayMode = "auto" | "windows" | "tab";

export type OverlayPlacement =
  | "top_left"
  | "top_right"
  | "bottom_left"
  | "bottom_right"
  | "centre";

export type RecommendationRefinement = "radiant" | "owned";

export type SteelPathFilter = "all" | "steelPath" | "normal";

export interface FissureFilter {
  tier: string;
  mission: string;
  location: string;
  steel_path: SteelPathFilter;
}

export type CyclePhase =
  | "earth_day"
  | "earth_night"
  | "cetus_day"
  | "cetus_night"
  | "vallis_warm"
  | "vallis_cold"
  | "cambion_fass"
  | "cambion_vome"
  | "duviri_sorrow"
  | "duviri_fear"
  | "duviri_joy"
  | "duviri_anger"
  | "duviri_envy"
  | "zariman_corpus"
  | "zariman_grineer";

export interface AlertSettings {
  fissure_notifications_enabled: boolean;
  fissure_filters: FissureFilter[];
  timers: CyclePhase[];
  timer_lead_secs: number;
}

export interface Settings {
  alerts: AlertSettings;
  windows_notifications_enabled: boolean;
  sound_notifications_enabled: boolean;
  discord_notifications_enabled: boolean;
  discord_webhook: string | null;
  discord_message_template: string;
  notification_only_background: boolean;
  price_ttl_minutes: number;
  world_state_interval_minutes: number;
  market_poll_minutes: number;
  market_auto_close: boolean;
  take_rank_into_account: boolean;
  include_founders_items: boolean | null;
  include_forma_ranks: boolean;
  show_full_inventory: boolean;
  stats_tab_enabled: boolean;
  overlays_enabled: boolean;
  overlay_relic_reward: boolean;
  overlay_relic_recommendation: boolean;
  overlay_riven: boolean;
  overlay_relic_reward_placement: OverlayPlacement;
  overlay_relic_recommendation_placement: OverlayPlacement;
  overlay_riven_placement: OverlayPlacement;
  overlay_recommendation_refinement: RecommendationRefinement;
  overlay_opacity: number;
  overlay_recommendation_count: number;
  overlay_mode: OverlayMode;
  overlay_only_while_game_active: boolean;
}
