import { create } from "zustand";
import type { Auction, MarketOrders, MarketSnapshot } from "@/types";

interface MarketListingsState {
  orders: MarketOrders;
  auctions: Auction[];
  at: string | null;
  setListings: (orders: MarketOrders, auctions: Auction[], at: string) => void;
  applySnapshot: (snapshot: MarketSnapshot) => void;
  clear: () => void;
}

const EMPTY = {
  orders: { items: [], rows: [] } satisfies MarketOrders,
  auctions: [] as Auction[],
  at: null,
};

export const useMarketListingsStore = create<MarketListingsState>((set) => ({
  ...EMPTY,
  setListings: (orders, auctions, at) => set({ orders, auctions, at }),
  applySnapshot: (snapshot) =>
    set((state) => ({
      orders: snapshot.orders ?? state.orders,
      auctions: snapshot.auctions ?? state.auctions,
      at: snapshot.at,
    })),
  clear: () => set(EMPTY),
}));
