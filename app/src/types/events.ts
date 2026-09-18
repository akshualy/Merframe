import type { Trade } from "@/types/stats";

export interface FissureInfo {
  node_id: string;
  node_name: string | null;
  mission_type: string;
  mission_name: string;
  planet: string | null;
  tier: string;
  steel_path: boolean;
  expiry: string;
  remaining_secs: number;
}

export interface InventorySummary {
  last_sync_oid: string;
  mr: number;
  plat: number;
  credits: number;
  endo: number;
  ducats: number;
  changes: number;
}

export type CoreEvent =
  | { InventoryUpdated: InventorySummary }
  | { RelicRewardScreen: { relic: string | null; rewards: string[] } }
  | {
      TradeCompleted: { at: string; partner: string | null; trade: Trade };
    }
  | { NewConversation: { channel: string; player: string } }
  | { FissureAlert: { fissure: FissureInfo } }
  | {
      TimerAlert: {
        name: string;
        next_state: string;
        ends_at: string;
        remaining_secs: number;
      };
    };

export interface CoreEventEnvelope {
  id: string;
  event: CoreEvent;
}
