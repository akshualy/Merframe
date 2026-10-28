import {
  Bell,
  ExternalLink,
  type LucideIcon,
  Star,
  TriangleAlert,
  X,
} from "lucide-react";
import type { ReactNode } from "react";
import { EmptyNote, Surface } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { slotMission, slotPlace } from "@/lib/arbitrations";
import { openExternal } from "@/lib/bridge";
import { dateTime } from "@/lib/format";
import type { ArbitrationNode, ArbitrationSlot } from "@/types";
import { HeroHeading } from "./schedule";

const SOURCE_URL = "https://browse.wf/arbys";
const TIERS_URL = "https://browse.wf/about";

function MarkRow({
  title,
  note,
  action,
  onRemove,
  busy,
}: {
  title: string;
  note: string;
  action: string;
  onRemove: () => void;
  busy: boolean;
}) {
  return (
    <li className="flex items-center gap-2 text-sm">
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate font-semibold">{title}</span>
        <Hint as="span">{note}</Hint>
      </span>
      <Button
        variant="ghost"
        size="icon"
        className="size-7"
        disabled={busy}
        aria-label={action}
        title={action}
        onClick={onRemove}
      >
        <X className="size-4" />
      </Button>
    </li>
  );
}

function MarkCard({
  icon,
  label,
  action,
  empty,
  children,
}: {
  icon: LucideIcon;
  label: string;
  action?: ReactNode;
  empty: string | null;
  children: ReactNode;
}) {
  return (
    <Surface className="gap-2 px-4 py-3">
      <HeroHeading icon={icon} label={label}>
        {action}
      </HeroHeading>
      {empty ? (
        <EmptyNote>{empty}</EmptyNote>
      ) : (
        <ul className="flex max-h-48 flex-col gap-2 overflow-y-auto">
          {children}
        </ul>
      )}
    </Surface>
  );
}

export function FavouriteNodesPanel({
  nodes,
  busy,
  onRemove,
}: {
  nodes: ArbitrationNode[];
  busy: boolean;
  onRemove: (nodeId: string) => void;
}) {
  return (
    <MarkCard
      icon={Star}
      label="Favourite nodes"
      empty={
        nodes.length === 0
          ? "You can star any node from the upcoming arbitrations and be notified about every arbitration on that node under Settings."
          : null
      }
    >
      {nodes.map((node) => (
        <MarkRow
          key={node.node_id}
          title={slotPlace(node)}
          note={slotMission(node)}
          action="Remove favourite node"
          busy={busy}
          onRemove={() => onRemove(node.node_id)}
        />
      ))}
    </MarkCard>
  );
}

export function RemindersPanel({
  slots,
  alertsOn,
  busy,
  onRemove,
}: {
  slots: ArbitrationSlot[];
  alertsOn: boolean;
  busy: boolean;
  onRemove: (slot: ArbitrationSlot) => void;
}) {
  return (
    <MarkCard
      icon={Bell}
      label="Reminders"
      empty={
        slots.length === 0
          ? `Use the bell next to an upcoming arbitration to be reminded of it once. ${
              alertsOn
                ? "Every arbitration matching your filters notifies on its own."
                : "Arbitrations matching your filters notify on their own once 'Notify about matching Arbitrations' is on under Settings."
            }`
          : null
      }
    >
      {slots.map((slot) => (
        <MarkRow
          key={slot.id}
          title={slotPlace(slot)}
          note={`${dateTime(slot.starts)}, ${slotMission(slot)}`}
          action="Remove reminder"
          busy={busy}
          onRemove={() => onRemove(slot)}
        />
      ))}
    </MarkCard>
  );
}

export function ForecastNotice({ children }: { children: string }) {
  return (
    <div className="text-warning flex items-center gap-2 text-sm">
      <TriangleAlert className="size-4 shrink-0" />
      {children}
    </div>
  );
}

function SourceLink({ url, children }: { url: string; children: string }) {
  return (
    <button
      type="button"
      className="text-accent inline-flex cursor-pointer items-center gap-1 hover:underline"
      onClick={() => openExternal(url)}
    >
      {children}
      <ExternalLink className="size-3" />
    </button>
  );
}

export function ForecastSource() {
  return (
    <Hint className="flex flex-wrap items-center gap-x-1">
      Schedule by
      <SourceLink url={SOURCE_URL}>browse.wf</SourceLink>, community tiers by
      the Arbitration Goons via
      <SourceLink url={TIERS_URL}>browse.wf/about</SourceLink>. A forecast, not
      a live confirmation from the game.
    </Hint>
  );
}
