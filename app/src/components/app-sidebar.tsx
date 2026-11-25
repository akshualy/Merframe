import type { LucideIcon } from "lucide-react";
import {
  Anvil,
  Boxes,
  ChartCandlestick,
  ChartPie,
  Globe,
  Handshake,
  Info,
  ListTodo,
  Pickaxe,
  PictureInPicture2,
  Settings,
} from "lucide-react";
import { NavLink } from "react-router";
import { BrandBar } from "@/components/brand-bar";
import { type GameIconName, LucideSlotGameIcon } from "@/components/game-icon";
import { cn } from "@/lib/utils";
import { useSettings } from "@/stores/app-store";

const NAV_GROUPS = [
  [
    { to: "/world", label: "World", icon: Globe },
    { to: "/reset-checklist", label: "Reset Checklist", icon: ListTodo },
    { to: "/arbitrations", label: "Arbitrations", icon: "arbitration" },
    { to: "/overlays", label: "Overlays", icon: PictureInPicture2 },
  ],
  [
    { to: "/inventory", label: "Inventory", icon: Boxes },
    { to: "/foundry", label: "Foundry", icon: Anvil },
    { to: "/resources", label: "Resources", icon: Pickaxe },
    { to: "/mastery", label: "Mastery", icon: "mastered" },
    { to: "/relics", label: "Relic Planner", icon: "relic" },
  ],
  [
    { to: "/rivens", label: "Riven Explorer", icon: "riven" },
    { to: "/market", label: "Market", icon: Handshake },
    { to: "/analytics", label: "Trading Analytics", icon: ChartCandlestick },
    { to: "/stats", label: "Stats", icon: ChartPie },
  ],
] as const;

const FOOTER_NAV = [
  { to: "/settings", label: "Settings", icon: Settings },
  { to: "/about", label: "About", icon: Info },
] as const;

function Item({
  to,
  label,
  icon: Icon,
}: {
  to: string;
  label: string;
  icon: LucideIcon | GameIconName;
}) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn(
          "text-foreground hover:bg-secondary/60 hover:text-accent flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-bold transition-colors duration-200",
          isActive && "bg-secondary text-primary hover:text-primary",
        )
      }
    >
      {typeof Icon === "string" ? (
        <LucideSlotGameIcon name={Icon} />
      ) : (
        <Icon className="size-4 shrink-0" />
      )}
      <span className="truncate">{label}</span>
    </NavLink>
  );
}

export function AppSidebar() {
  const statsTab = useSettings().stats_tab_enabled;

  return (
    <aside className="bg-card flex w-60 shrink-0 flex-col border-r">
      <BrandBar />
      <nav className="flex flex-1 flex-col gap-2 overflow-y-auto p-3">
        {NAV_GROUPS.map((group) => (
          <div
            key={group[0].to}
            className="flex flex-col gap-1 not-first:border-t not-first:pt-2"
          >
            {group
              .filter((entry) => statsTab || entry.to !== "/stats")
              .map((entry) => (
                <Item key={entry.to} {...entry} />
              ))}
          </div>
        ))}
      </nav>
      <div className="flex flex-col gap-1 border-t p-3">
        {FOOTER_NAV.map((entry) => (
          <Item key={entry.to} {...entry} />
        ))}
      </div>
    </aside>
  );
}
