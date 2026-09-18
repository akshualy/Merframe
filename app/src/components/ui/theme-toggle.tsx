import { MoonIcon, SunIcon } from "lucide-react";
import { useTheme } from "next-themes";

import { Button } from "@/components/ui/button";

export function ModeToggle() {
  const { resolvedTheme, setTheme } = useTheme();

  return (
    <Button
      variant="outline"
      size="icon"
      className="rounded-full"
      onClick={() => setTheme(resolvedTheme === "light" ? "dark" : "light")}
    >
      <SunIcon className="size-4 scale-100 transition-transform dark:scale-0" />
      <MoonIcon className="absolute size-4 scale-0 transition-transform dark:scale-100" />
      <span className="sr-only">Toggle theme</span>
    </Button>
  );
}
