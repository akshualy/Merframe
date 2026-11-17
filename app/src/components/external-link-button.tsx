import { ExternalLink } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { openExternal } from "@/lib/bridge";

export interface ExternalLinkTarget {
  label: string;
  url: string;
}

export function ExternalLinkButton({
  url,
  className,
  children,
}: {
  url: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <Button
      variant="outline"
      size="sm"
      className={className}
      onClick={() => openExternal(url)}
    >
      {children}
      <ExternalLink className="size-3.5" />
    </Button>
  );
}
