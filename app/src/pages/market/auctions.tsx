import type { ColumnDef } from "@tanstack/react-table";
import { Ellipsis, RefreshCw, Wrench } from "lucide-react";
import { useMemo, useState } from "react";
import { DataTable } from "@/components/data-table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { api, reportError } from "@/lib/bridge";
import { dateTime, num } from "@/lib/format";
import { rivenAuctionUrl } from "@/lib/rivens";
import { ListingDialog, type ListingForm } from "@/pages/rivens/listing-dialog";
import type { Auction, MarketItem } from "@/types";

function titleCase(slug: string): string {
  return slug
    .split(/[_\s]+/)
    .filter((word) => word.length > 0)
    .map((word) =>
      word
        .split("-")
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join("-"),
    )
    .join(" ");
}

function auctionName(auction: Auction, items: MarketItem[]): string {
  const slug = auction.item.weapon_url_name;
  const weapon = items.find((item) => item.slug === slug)?.name;
  return `${weapon ?? titleCase(slug)} ${titleCase(auction.item.name)}`;
}

function auctionForm(auction: Auction): ListingForm {
  return {
    direct: auction.is_direct_sell,
    visible: auction.visible,
    rank: auction.item.mod_rank,
    sellingPrice: String(auction.starting_price),
    startingPrice: String(auction.starting_price),
    buyoutPrice: String(auction.buyout_price ?? 0),
    minReputation: String(auction.minimal_reputation),
    note: auction.note ?? "",
  };
}

export function AuctionsTable({
  auctions,
  items,
  onRefresh,
  onSetVisibility,
  runMarketAction,
}: {
  auctions: Auction[];
  items: MarketItem[];
  onRefresh: () => void;
  onSetVisibility: (visible: boolean) => void;
  runMarketAction: (
    promise: Promise<unknown>,
    message: string,
  ) => Promise<void>;
}) {
  const [editing, setEditing] = useState<Auction | null>(null);

  const columns = useMemo<ColumnDef<Auction>[]>(
    () => [
      {
        id: "riven",
        header: "Riven",
        enableHiding: false,
        accessorFn: (row) => auctionName(row, items),
        cell: ({ row }) => (
          <span className="flex items-center gap-2">
            <span className="font-medium">
              {auctionName(row.original, items)}
            </span>
            <Badge variant="muted">rank {row.original.item.mod_rank}</Badge>
            <Badge variant="muted">{row.original.item.re_rolls} rolls</Badge>
          </span>
        ),
      },
      {
        id: "kind",
        header: "Kind",
        accessorFn: (row) => (row.is_direct_sell ? "direct sell" : "auction"),
        cell: ({ row }) => (
          <Badge
            variant={row.original.is_direct_sell ? "default" : "secondary"}
          >
            {row.original.is_direct_sell ? "direct sell" : "auction"}
          </Badge>
        ),
      },
      {
        accessorKey: "starting_price",
        header: "Starting",
        meta: { numeric: true, label: "Starting price" },
        cell: ({ row }) => (
          <span className="text-primary font-bold">
            {num(row.original.starting_price)}
          </span>
        ),
      },
      {
        accessorKey: "buyout_price",
        header: "Buyout",
        meta: { numeric: true, label: "Buyout price" },
        cell: ({ row }) =>
          row.original.buyout_price === null
            ? "-"
            : num(row.original.buyout_price),
      },
      {
        accessorKey: "visible",
        header: "Visible",
        cell: ({ row }) => (
          <span className="flex items-center gap-2">
            {row.original.visible ? (
              <Badge variant="accent">visible</Badge>
            ) : (
              <Badge variant="muted">hidden</Badge>
            )}
            {row.original.private && <Badge variant="muted">private</Badge>}
            {row.original.closed && <Badge variant="muted">closed</Badge>}
          </span>
        ),
      },
      {
        accessorKey: "created",
        header: "Created",
        cell: ({ row }) => dateTime(row.original.created),
      },
      {
        accessorKey: "updated",
        header: "Updated",
        cell: ({ row }) => dateTime(row.original.updated),
      },
      {
        id: "actions",
        header: "",
        enableSorting: false,
        enableHiding: false,
        cell: ({ row }) => (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                aria-label="Auction actions"
              >
                <Ellipsis className="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onSelect={async () => {
                  try {
                    await api.openUrl(
                      rivenAuctionUrl(row.original.item.weapon_url_name),
                    );
                  } catch (error) {
                    reportError(error);
                  }
                }}
              >
                Compare on the market
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={() => setEditing(row.original)}>
                Edit auction
              </DropdownMenuItem>
              <DropdownMenuItem
                onSelect={() =>
                  runMarketAction(
                    api.marketSetAuctionVisible(
                      row.original.id,
                      !row.original.visible,
                    ),
                    row.original.visible ? "Auction hidden" : "Auction shown",
                  )
                }
              >
                {row.original.visible ? "Hide auction" : "Show auction"}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                variant="destructive"
                disabled={row.original.closed}
                onSelect={() =>
                  runMarketAction(
                    api.marketCloseAuction(row.original.id),
                    "Auction closed",
                  )
                }
              >
                Close auction
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        ),
      },
    ],
    [items, runMarketAction],
  );

  return (
    <>
      <DataTable
        tableId="marketAuctions"
        columns={columns}
        data={auctions}
        toolbar={
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
                  disabled={auctions.length === 0}
                  onSelect={() => onSetVisibility(true)}
                >
                  Show all auctions
                </DropdownMenuItem>
                <DropdownMenuItem
                  disabled={auctions.length === 0}
                  onSelect={() => onSetVisibility(false)}
                >
                  Hide all auctions
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button variant="outline" size="sm" onClick={onRefresh}>
              <RefreshCw className="size-4" />
              Refresh
            </Button>
          </>
        }
        searchPlaceholder="Filter auctions"
        initialSorting={[{ id: "updated", desc: true }]}
        rowKey={(row) => row.id}
        emptyMessage="No riven auctions on this account."
      />
      {editing && (
        <ListingDialog
          key={editing.id}
          title="Edit Auction"
          description={auctionName(editing, items)}
          initial={auctionForm(editing)}
          submitLabel="Save"
          onSubmit={(choices) =>
            runMarketAction(
              api.marketEditAuction(editing.id, choices),
              "Auction updated",
            )
          }
          onClose={() => setEditing(null)}
        />
      )}
    </>
  );
}
