import { Ellipsis, RefreshCw, Wrench } from "lucide-react";
import type { ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

export function ListingToolbar({
  noun,
  empty,
  onSetVisibility,
  onRefresh,
  children,
}: {
  noun: string;
  empty: boolean;
  onSetVisibility: (visible: boolean) => void;
  onRefresh: () => void;
  children?: ReactNode;
}) {
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm">
            <Wrench className="size-4" />
            Bulk Actions
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem
            disabled={empty}
            onSelect={() => onSetVisibility(true)}
          >
            Show all {noun}
          </DropdownMenuItem>
          <DropdownMenuItem
            disabled={empty}
            onSelect={() => onSetVisibility(false)}
          >
            Hide all {noun}
          </DropdownMenuItem>
          {children}
        </DropdownMenuContent>
      </DropdownMenu>
      <Button variant="outline" size="sm" onClick={onRefresh}>
        <RefreshCw className="size-4" />
        Refresh
      </Button>
    </>
  );
}

export function RowActions({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          aria-label={label}
        >
          <Ellipsis className="size-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">{children}</DropdownMenuContent>
    </DropdownMenu>
  );
}

export function VisibleBadge({ visible }: { visible: boolean }) {
  return visible ? (
    <Badge variant="accent">visible</Badge>
  ) : (
    <Badge variant="muted">hidden</Badge>
  );
}
