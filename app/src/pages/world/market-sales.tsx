import { Store } from "lucide-react";
import { Amount } from "@/components/currency";
import { EmptyNote } from "@/components/page";
import { num, secondsUntil } from "@/lib/format";
import { timeLeft } from "@/lib/world";
import type { MarketOffer } from "@/types";
import { Group, InfoRow, Panel } from "./panels";

function MarketOffers({
  label,
  offers,
  now,
}: {
  label?: string | undefined;
  offers: MarketOffer[];
  now: number;
}) {
  if (offers.length === 0) {
    return null;
  }
  return (
    <Group label={label}>
      {offers.map((offer) => {
        const left = `${timeLeft(secondsUntil(offer.ends, now))} left`;
        return (
          <InfoRow
            key={offer.name}
            label={offer.name}
            ownership={offer.ownership}
            hint={
              offer.discount_percent > 0
                ? `-${offer.discount_percent}%, ${left}`
                : left
            }
            value={
              offer.platinum > 0 ? (
                <Amount currency="platinum" plain>
                  {num(offer.platinum)}
                </Amount>
              ) : (
                <Amount currency="credits" plain>
                  {num(offer.credits)}
                </Amount>
              )
            }
          />
        );
      })}
    </Group>
  );
}

export function MarketSalesPanel({
  sales,
  now,
}: {
  sales: MarketOffer[];
  now: number;
}) {
  return (
    <Panel icon={Store} title="Market Sales">
      {sales.length === 0 ? (
        <EmptyNote>The Market has no sale right now.</EmptyNote>
      ) : (
        <>
          <MarketOffers
            offers={sales.filter((offer) => offer.discount_percent > 0)}
            now={now}
          />
          <MarketOffers
            label="Limited time"
            offers={sales.filter((offer) => offer.discount_percent === 0)}
            now={now}
          />
        </>
      )}
    </Panel>
  );
}
