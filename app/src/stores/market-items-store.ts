import { create } from "zustand";
import type { MarketItem } from "@/types";

interface MarketItemsState {
  items: MarketItem[];
  bySlug: ReadonlyMap<string, MarketItem>;
  setItems: (items: MarketItem[]) => void;
}

export const useMarketItemsStore = create<MarketItemsState>((set) => ({
  items: [],
  bySlug: new Map(),
  setItems: (items) =>
    set({ items, bySlug: new Map(items.map((item) => [item.slug, item])) }),
}));
