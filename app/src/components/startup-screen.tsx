import { RotateCcw } from "lucide-react";
import Merframe from "@/components/icons/merframe";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { usePageQuote } from "@/lib/quotes";

export function StartupScreen({
  error,
  onRetry,
}: {
  error: string | null;
  onRetry: () => void;
}) {
  const quote = usePageQuote("startup");

  return (
    <div
      className="bg-background flex flex-col h-full"
      role={error ? "alert" : "status"}
      aria-live={error ? "assertive" : "polite"}
    >
      <header className="bg-card flex h-14 shrink-0 items-center gap-2 border-b px-4">
        <Merframe className="text-primary size-7" />
        <span className="text-lg font-bold tracking-tight">Merframe</span>
      </header>

      <main className="flex h-full items-center justify-center gap-2 p-8">
        <div className="flex flex-col gap-2 w-full max-w-lg">
          <div className="flex items-center gap-5 animate-pulse">
            <Merframe className="text-primary size-12" />
            <div className="pt-1">
              <h1 className="text-primary mt-1 text-2xl font-bold">
                {error ? "Merframe could not start" : "Waking ship systems"}
              </h1>
            </div>
          </div>

          {error && (
            <Button className="mt-8" variant="outline" onClick={onRetry}>
              <RotateCcw />
              Try Again
            </Button>
          )}

          <div className="flex flex-col gap-2">
            <span className="text-sm">{quote.line}</span>
            <Hint as="span">{quote.speaker}</Hint>
          </div>
        </div>
      </main>
    </div>
  );
}
