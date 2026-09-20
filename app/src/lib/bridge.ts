import { toast } from "sonner";
import type {
  Auction,
  AuctionPatch,
  CommandError,
  ComparedStat,
  CraftDetails,
  FoundryTab,
  GameStatus,
  InventoryTab,
  ListingChoices,
  MarketAccount,
  MarketItem,
  MarketPresence,
  MarketStatus,
  MasteryOrdering,
  MasteryTab,
  NewOrder,
  Order,
  OrderBook,
  OrderPatch,
  OrderRow,
  OverlayState,
  RelicPlannerTab,
  RelicSource,
  ResourceScope,
  ResourceSource,
  ResourcesTab,
  RewardScreen,
  RivenComparables,
  RivensTab,
  Settings,
  StatsTab,
  WorldStateView,
} from "@/types";

type Args = Record<string, unknown> | undefined;

async function call<T>(command: string, args?: Args): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

export const events = {
  appReady: "app-ready",
  appError: "app-error",
  coreEvent: "core-event",
  inventoryUpdated: "inventory-updated",
  statusUpdated: "status-updated",
  worldStateUpdated: "worldstate-updated",
  pricesUpdated: "prices-updated",
  rivenDataUpdated: "riven-data-updated",
  marketUpdated: "market-updated",
  marketAutoClosed: "market-auto-closed",
  marketSignedOut: "market-signed-out",
  marketPresence: "market-presence",
  overlayState: "overlay-state",
} as const;

export type AppEvent = (typeof events)[keyof typeof events];

export type Unlisten = () => void;

export async function listenTo<T>(
  event: AppEvent,
  handler: (payload: T) => void,
): Promise<Unlisten> {
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, (message) => handler(message.payload));
}

function commandError(error: unknown): CommandError | null {
  if (error && typeof error === "object" && "message" in error) {
    return error as CommandError;
  }
  return null;
}

export function isStarting(error: unknown): boolean {
  return commandError(error)?.code === "starting";
}

export function errorMessage(error: unknown): string {
  const failure = commandError(error);
  if (failure) {
    return failure.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

export function reportError(cause: unknown) {
  toast.error(errorMessage(cause));
}

export function logError(context: string, error: unknown) {
  console.error(`${context}: ${errorMessage(error)}`, error);
}

export const api = {
  inventoryTab: () => call<InventoryTab>("inventory_tab"),
  foundryTab: () => call<FoundryTab>("foundry_tab"),
  craftTree: (uniqueName: string) =>
    call<CraftDetails>("craft_tree", { uniqueName }),
  masteryTab: (
    ordering?: MasteryOrdering,
    includeFounders?: boolean | null,
    includeFormaRanks?: boolean,
  ) =>
    call<MasteryTab>("mastery_tab", {
      ordering: ordering ?? null,
      includeFounders: includeFounders ?? null,
      includeFormaRanks: includeFormaRanks ?? null,
    }),
  resourcesTab: (source: ResourceSource, scope: ResourceScope) =>
    call<ResourcesTab>("resources_tab", { source, scope }),
  relicPlannerTab: (squadSize?: number, onlyOwned?: boolean) =>
    call<RelicPlannerTab>("relic_planner_tab", {
      squadSize: squadSize ?? null,
      onlyOwned: onlyOwned ?? null,
    }),
  rivensTab: () => call<RivensTab>("rivens_tab"),
  rivenComparables: (weaponSlug: string, shown: ComparedStat[]) =>
    call<RivenComparables | null>("riven_comparables", { weaponSlug, shown }),
  statsTab: (sinceMs?: number) =>
    call<StatsTab>("stats_tab", { sinceMs: sinceMs ?? null }),
  toggleFavourite: (uniqueName: string) =>
    call<boolean>("toggle_favourite", { uniqueName }),
  relicsFor: (partUniqueName: string) =>
    call<RelicSource[]>("relics_for", { partUniqueName }),
  recommend: (rewards: string[]) =>
    call<RewardScreen>("recommend", { rewards }),
  gameStatus: () => call<GameStatus>("game_status"),
  overlayState: () => call<OverlayState>("overlay_state"),
  overlayPageReady: () => call<void>("overlay_page_ready"),
  overlayContentShrank: () => call<void>("overlay_content_shrank"),
  rescanInventory: () => call<GameStatus>("rescan_inventory"),
  exportBundle: () => call<string | null>("export"),
  pickLogFile: () => call<string | null>("pick_log_file"),
  settingsGet: () => call<Settings>("settings_get"),
  settingsSet: (settings: Settings) =>
    call<Settings>("settings_set", { settings }),
  testNotifications: (settings: Settings) =>
    call<void>("test_notifications", { settings }),
  marketLogin: (email: string, password: string) =>
    call<MarketAccount>("market_login", { email, password }),
  marketLogout: () => call<void>("market_logout"),
  marketMyOrders: () => call<OrderRow[]>("market_my_orders"),
  marketPostOrder: (order: NewOrder) =>
    call<Order>("market_post_order", { order }),
  marketUpdateOrder: (id: string, patch: OrderPatch) =>
    call<Order>("market_update_order", { id, patch }),
  marketCloseOrder: (id: string, quantity: number) =>
    call<void>("market_close_order", { id, quantity }),
  marketDeleteOrder: (id: string) => call<Order>("market_delete_order", { id }),
  marketActivity: () => call<void>("market_activity"),
  marketPresence: () => call<MarketPresence>("market_presence"),
  marketSetPresence: (status: MarketStatus | null, auto: boolean) =>
    call<MarketPresence>("market_set_presence", { status, auto }),
  marketRemoveAll: () => call<number>("market_remove_all"),
  marketFixOrders: (id?: string) => call<number>("market_fix_orders", { id }),
  marketSetVisibility: (visible: boolean) =>
    call<number>("market_set_visibility", { visible }),
  marketItems: () => call<MarketItem[]>("market_items"),
  marketItemOrders: (slug: string) =>
    call<OrderBook>("market_item_orders", { slug }),
  marketPostRiven: (itemId: string, choices: ListingChoices) =>
    call<string>("market_post_riven", { itemId, choices }),
  marketMyAuctions: () => call<Auction[]>("market_my_auctions"),
  marketUpdateAuction: (id: string, patch: AuctionPatch) =>
    call<Auction>("market_update_auction", { id, patch }),
  marketCloseAuction: (id: string) =>
    call<Auction>("market_close_auction", { id }),
  marketSetAuctionsVisibility: (visible: boolean) =>
    call<void>("market_set_auctions_visibility", { visible }),
  worldstate: () => call<WorldStateView>("worldstate"),
  refreshPrices: () => call<number>("refresh_prices"),
  openUrl: (url: string) => call<void>("open_url", { url }),
  openDataFolder: () => call<void>("open_data_folder"),
  openGameLogFolder: () => call<void>("open_game_log_folder"),
  itemImage: (imageName: string) => call<string>("item_image", { imageName }),
  prefetchImages: (names: string[]) =>
    call<number>("prefetch_images", { names }),
};
