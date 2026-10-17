import Merframe from "@/components/icons/merframe";

export function BrandBar() {
  return (
    <header className="bg-card flex h-14 shrink-0 items-center gap-2 border-b px-4">
      <Merframe className="text-primary size-7" />
      <span className="text-foreground text-lg font-bold tracking-tight">
        Merframe
      </span>
    </header>
  );
}
