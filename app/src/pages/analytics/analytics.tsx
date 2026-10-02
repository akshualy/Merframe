import { Page, Quoted } from "@/components/page";
import { usePageQuote } from "@/lib/quotes";
import { MarketMovers } from "./market-movers";
import { TradeHistory } from "./trade-history";

export function AnalyticsPage() {
  const quote = usePageQuote("analytics");

  return (
    <Page title="Trading Analytics" description={<Quoted quote={quote} />}>
      <MarketMovers />
      <TradeHistory />
    </Page>
  );
}
