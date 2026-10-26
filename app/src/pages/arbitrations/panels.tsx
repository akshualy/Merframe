import { ExternalLink, TriangleAlert } from "lucide-react";
import { Hint } from "@/components/ui/hint";
import { openExternal } from "@/lib/bridge";

const SOURCE_URL = "https://browse.wf/arbys";
const TIERS_URL = "https://browse.wf/about";

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
