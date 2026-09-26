export type OrderType = "buy" | "sell";

export interface MarketUser {
  id: string;
  ingameName: string;
  slug: string;
  status?: string;
  locale: string;
}

export interface Order {
  id: string;
  type: OrderType;
  platinum: number;
  quantity: number;
  perTrade?: number;
  subtype?: string;
  rank?: number;
  amberStars?: number;
  cyanStars?: number;
  visible: boolean;
  updatedAt: string;
  itemId: string;
  user?: MarketUser;
}

export interface OrderBook {
  sell: Order[];
  buy: Order[];
}

export type MarketStatus = "offline" | "online" | "ingame" | "invisible";

export interface MarketPresence {
  status: MarketStatus | null;
  auto: boolean;
}

export type MarketCategory =
  | "parts"
  | "relics"
  | "mods"
  | "arcanes"
  | "sets"
  | "misc";

export interface OrderRow {
  id: string;
  order_type: OrderType;
  item_id: string;
  slug: string;
  name: string;
  thumb: string;
  category: MarketCategory;
  platinum: number;
  quantity: number;
  per_trade: number | null;
  rank: number | null;
  subtype: string | null;
  visible: boolean;
  updated_at: string;
  owned: number;
  show_warning: boolean;
  lowest: number | null;
  lowest_from_rank_zero: boolean;
}

export interface MarketItem {
  id: string;
  slug: string;
  name: string;
  thumb: string;
  ducats: number | null;
  tradable: boolean | null;
  bulk_tradable: boolean | null;
  max_rank: number | null;
  tags: string[];
  subtypes?: string[];
  max_amber_stars?: number | null;
  max_cyan_stars?: number | null;
}

export interface NewOrder {
  item_id: string;
  order_type: OrderType;
  platinum: number;
  quantity: number;
  visible?: boolean;
  per_trade?: number;
  rank?: number;
  subtype?: string;
  amber_stars?: number;
  cyan_stars?: number;
}

export interface OrderPatch {
  platinum?: number;
  quantity?: number;
  visible?: boolean;
}

export interface RivenAttributeInstance {
  value: number;
  positive: boolean;
  url_name: string;
}

export interface AuctionItem {
  attributes: RivenAttributeInstance[];
  polarity: string;
  mod_rank: number;
  name: string;
  re_rolls: number;
  mastery_level: number;
  weapon_url_name: string;
}

export interface Auction {
  id: string;
  buyout_price: number | null;
  starting_price: number;
  note?: string;
  item: AuctionItem;
  private: boolean;
  visible: boolean;
  closed: boolean;
  is_direct_sell: boolean;
  created: string;
  updated: string;
}

export interface MarketAutoClose {
  item: string;
  quantity: number;
  kind: "auction" | "sell" | "buy";
}

export interface MarketSnapshot {
  orders: OrderRow[] | null;
  auctions: Auction[] | null;
  at: string;
}

export interface AuctionPatch {
  starting_price?: number;
  buyout_price?: number;
  note?: string;
  visible?: boolean;
}
