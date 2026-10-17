import { Page } from "@/components/page";
import { MarketMovers } from "./market-movers";
import { TradeHistory } from "./trade-history";

export function AnalyticsPage() {
  return (
    <Page title="Trading Analytics" quote="analytics">
      <MarketMovers />
      <TradeHistory />
    </Page>
  );
}
