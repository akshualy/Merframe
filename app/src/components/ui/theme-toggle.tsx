import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { useTheme } from "next-themes";

import { Button } from "@/components/ui/button";

type Step = { next: string; label: string; Icon: typeof SunIcon };

const SYSTEM: Step = {
  next: "dark",
  label: "Theme follows the system",
  Icon: MonitorIcon,
};
const CYCLE: Record<string, Step> = {
  dark: { next: "light", label: "Dark theme", Icon: MoonIcon },
  light: { next: "system", label: "Light theme", Icon: SunIcon },
  system: SYSTEM,
};

export function ModeToggle() {
  const { theme = "system", setTheme } = useTheme();
  const current = CYCLE[theme] ?? SYSTEM;

  return (
    <Button
      variant="outline"
      size="icon"
      className="rounded-full"
      aria-label={current.label}
      title={current.label}
      onClick={() => setTheme(current.next)}
    >
      <current.Icon className="size-4" />
    </Button>
  );
}
