export interface MarketAccount {
  ingame_name: string;
  slug: string;
  tier: string;
  mastery_rank: number;
}

export type InventorySource = "none" | "live" | "cached";

export interface OverlaySupport {
  windows_possible: boolean;
  detail: string;
}

export interface GameStatus {
  game_detected: boolean;
  pid: number | null;
  scanning: boolean;
  source: InventorySource;
  last_scan_at: string | null;
  last_scan_error: string | null;
  last_sync_oid: string | null;
  last_sync_at: string | null;
  inventory_age_secs: number | null;
  world_state_at: string | null;
  price_table_at: string | null;
  log_file: string | null;
  log_attached: boolean;
  market_account: MarketAccount | null;
  market_unread: number;
  overlay_support: OverlaySupport;
}

export interface CommandError {
  message: string;
  code: string | null;
}
