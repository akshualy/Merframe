import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { OrderType } from "@/types";

export interface ListingStars {
  amber: number;
  cyan: number;
}

export interface ListingRequest {
  slug: string;
  side: OrderType;
  rank: number | null;
  subtype: string | null;
  stars: ListingStars | null;
}

interface MarketPanelState {
  open: boolean;
  request: ListingRequest | null;
  show: () => void;
  hide: () => void;
  openListing: (
    slug: string,
    side: OrderType,
    rank?: number | null,
    subtype?: string | null,
    stars?: ListingStars | null,
  ) => void;
  takeRequest: () => void;
}

export const useMarketPanelStore = create<MarketPanelState>()(
  persist(
    (set) => ({
      open: false,
      request: null,
      show: () => set({ open: true }),
      hide: () => set({ open: false }),
      openListing: (slug, side, rank = null, subtype = null, stars = null) =>
        set({ open: true, request: { slug, side, rank, subtype, stars } }),
      takeRequest: () => set({ request: null }),
    }),
    {
      name: "merframe.marketPanel",
      partialize: (state) => ({ open: state.open }),
    },
  ),
);
