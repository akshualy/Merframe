import type { LucideIcon } from "lucide-react";
import {
  BarChart3,
  Boxes,
  Globe,
  GraduationCap,
  Hammer,
  Info,
  Monitor,
  Package,
  Settings,
  ShoppingCart,
  Sparkles,
  Target,
} from "lucide-react";
import { NavLink } from "react-router";
import Merframe from "@/components/icons/merframe";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app-store";

const NAV = [
  { to: "/world", label: "World", icon: Globe },
  { to: "/foundry", label: "Foundry", icon: Hammer },
  { to: "/inventory", label: "Inventory", icon: Boxes },
  { to: "/relics", label: "Relic Planner", icon: Package },
  { to: "/rivens", label: "Riven Explorer", icon: Sparkles },
  { to: "/overlays", label: "Overlays", icon: Monitor },
  { to: "/mastery", label: "Mastery", icon: GraduationCap },
  { to: "/resources", label: "Resources", icon: Target },
  { to: "/market", label: "warframe.market", icon: ShoppingCart },
  { to: "/stats", label: "Stats", icon: BarChart3 },
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
  icon: LucideIcon;
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
      <Icon className="size-4 shrink-0" />
      <span className="truncate">{label}</span>
    </NavLink>
  );
}

export function AppSidebar() {
  const { settings } = useAppStore();
  const statsTab = settings?.stats_tab_enabled ?? true;

  return (
    <aside className="bg-card flex w-60 shrink-0 flex-col border-r">
      <div className="flex h-14 items-center gap-2 border-b px-4">
        <Merframe className="text-primary size-7" />
        <span className="text-foreground text-lg font-bold tracking-tight">
          Merframe
        </span>
      </div>
      <nav className="flex flex-1 flex-col gap-1 overflow-y-auto p-3">
        {NAV.filter((entry) => statsTab || entry.to !== "/stats").map(
          (entry) => (
            <Item key={entry.to} {...entry} />
          ),
        )}
      </nav>
      <div className="flex flex-col gap-1 border-t p-3">
        {FOOTER_NAV.map((entry) => (
          <Item key={entry.to} {...entry} />
        ))}
      </div>
    </aside>
  );
}
