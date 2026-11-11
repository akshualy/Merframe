import { ItemImage } from "@/components/item-image";
import { Section } from "@/components/page";
import { Hint } from "@/components/ui/hint";
import { counted, num } from "@/lib/format";
import { capitalize } from "@/lib/world";
import type {
  KuvaWeeklySummary,
  ResetChecklistVendor,
  ResetWindow,
  VendorWeeklyOffer,
} from "@/types";
import { ResetChecklistCountdown } from "./reset-countdown";
import { ResetChecklistTile, ResetChecklistTileGrid } from "./tiles";

function VendorWeeklyOfferLine({ offer }: { offer: VendorWeeklyOffer }) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="flex min-w-0 items-center gap-1.5">
        {offer.kind === "kuva" ? (
          `${num(offer.amount)} Kuva`
        ) : (
          <>
            <ItemImage imageName={offer.image_name} size={20} />
            <span className="truncate">
              {capitalize(offer.colour)} Archon Shard
            </span>
          </>
        )}
      </span>
      <span className="shrink-0">
        {num(offer.bought)}/{num(offer.limit)}
      </span>
    </div>
  );
}

export function ResetChecklistVendorPurchases({
  vendors,
  kuva,
  reset,
  now,
}: {
  vendors: ResetChecklistVendor[];
  kuva: KuvaWeeklySummary;
  reset: ResetWindow;
  now: number;
}) {
  return (
    <Section
      title="Shops"
      description={`${num(kuva.bought)} of ${num(kuva.available)} Kuva bought this week`}
      action={<ResetChecklistCountdown window={reset} now={now} />}
    >
      <ResetChecklistTileGrid>
        {vendors.map((vendor) => (
          <ResetChecklistTile key={vendor.name}>
            <span className="font-medium">{vendor.name}</span>
            <Hint as="span">
              {[
                vendor.place,
                `${counted(vendor.offers_bought, "offer")} bought`,
              ]
                .filter(Boolean)
                .join(", ")}
            </Hint>
            {vendor.weekly_offer && (
              <VendorWeeklyOfferLine offer={vendor.weekly_offer} />
            )}
          </ResetChecklistTile>
        ))}
      </ResetChecklistTileGrid>
    </Section>
  );
}
