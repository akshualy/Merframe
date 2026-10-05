import { beforeEach, describe, expect, it } from "vitest";
import { useMarketListingsStore } from "@/stores/market-listings-store";
import type { Auction, MarketOrders } from "@/types";

const AUCTION = { id: "auction-1" } as Partial<Auction> as Auction;

describe("market listings store", () => {
  beforeEach(() => {
    useMarketListingsStore.getState().clear();
  });

  it("keeps the side a snapshot leaves out", () => {
    const { setListings, applySnapshot } = useMarketListingsStore.getState();
    setListings({ items: [], rows: [] }, [AUCTION], "2026-10-04T10:00:00Z");
    const fresh: MarketOrders = { items: [], rows: [] };
    applySnapshot({
      orders: fresh,
      auctions: null,
      at: "2026-10-04T10:01:00Z",
    });
    const state = useMarketListingsStore.getState();
    expect(state.orders).toBe(fresh);
    expect(state.auctions).toEqual([AUCTION]);
    expect(state.at).toBe("2026-10-04T10:01:00Z");

    applySnapshot({
      orders: null,
      auctions: [],
      at: "2026-10-04T10:02:00Z",
    });
    expect(useMarketListingsStore.getState().orders).toBe(fresh);
    expect(useMarketListingsStore.getState().auctions).toEqual([]);
  });
});
