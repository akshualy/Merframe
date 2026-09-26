import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { MarketItem, OrderType } from "@/types";

export interface ListingRequest {
  slug: string;
  side: OrderType;
  rank: number | null;
  nonce: number;
}

interface MarketPanelState {
  open: boolean;
  items: MarketItem[];
  request: ListingRequest | null;
  show: () => void;
  hide: () => void;
  setItems: (items: MarketItem[]) => void;
  openListing: (slug: string, side: OrderType, rank?: number | null) => void;
}

export const useMarketPanelStore = create<MarketPanelState>()(
  persist(
    (set) => ({
      open: false,
      items: [],
      request: null,
      show: () => set({ open: true }),
      hide: () => set({ open: false }),
      setItems: (items) => set({ items }),
      openListing: (slug, side, rank = null) =>
        set({ open: true, request: { slug, side, rank, nonce: Date.now() } }),
    }),
    {
      name: "merframe.marketPanel",
      partialize: (state) => ({ open: state.open }),
    },
  ),
);
