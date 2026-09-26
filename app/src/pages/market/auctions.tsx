import type { ColumnDef } from "@tanstack/react-table";
import { Ellipsis, RefreshCw, Wrench } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { DataTable } from "@/components/data-table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api, reportError } from "@/lib/bridge";
import { dateTime, num } from "@/lib/format";
import { rivenAuctionUrl } from "@/lib/rivens";
import type { Auction, AuctionPatch, MarketItem } from "@/types";

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
  runMarketAction: (promise: Promise<unknown>, message: string) => void;
}) {
  const [editing, setEditing] = useState<Auction | null>(null);
  const [starting, setStarting] = useState("");
  const [buyout, setBuyout] = useState("");
  const [note, setNote] = useState("");

  const handleEdit = useCallback((auction: Auction) => {
    setEditing(auction);
    setStarting(String(auction.starting_price));
    setBuyout(
      auction.buyout_price === null ? "" : String(auction.buyout_price),
    );
    setNote(auction.note ?? "");
  }, []);

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
              <DropdownMenuItem onSelect={() => handleEdit(row.original)}>
                Edit price
              </DropdownMenuItem>
              <DropdownMenuItem
                onSelect={() =>
                  runMarketAction(
                    api.marketUpdateAuction(row.original.id, {
                      visible: !row.original.visible,
                    }),
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
    [items, runMarketAction, handleEdit],
  );

  const handleSubmit = useCallback(() => {
    if (!editing) {
      return;
    }
    const patch: AuctionPatch = {
      starting_price: Math.max(1, Number(starting) || editing.starting_price),
      note,
    };
    const wanted = Number(buyout);
    if (buyout.trim() !== "" && wanted > 0) {
      patch.buyout_price = wanted;
    }
    runMarketAction(
      api.marketUpdateAuction(editing.id, patch),
      "Auction updated",
    );
    setEditing(null);
  }, [editing, starting, buyout, note, runMarketAction]);

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
        pageSize={25}
        emptyMessage="No riven auctions on this account."
      />
      <Dialog
        open={editing !== null}
        onOpenChange={(open) => !open && setEditing(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Auction</DialogTitle>
            <DialogDescription>
              {editing ? auctionName(editing, items) : ""}
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor="auction-starting">Starting price</Label>
                <Input
                  id="auction-starting"
                  type="number"
                  min={1}
                  value={starting}
                  onChange={(e) => setStarting(e.target.value)}
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="auction-buyout">Buyout price</Label>
                <Input
                  id="auction-buyout"
                  type="number"
                  min={0}
                  value={buyout}
                  onChange={(e) => setBuyout(e.target.value)}
                  placeholder="none"
                />
              </div>
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="auction-note">Note</Label>
              <Input
                id="auction-note"
                value={note}
                onChange={(e) => setNote(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditing(null)}>
              Cancel
            </Button>
            <Button onClick={handleSubmit}>Save</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
