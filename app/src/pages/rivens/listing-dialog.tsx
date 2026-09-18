import { useCallback, useState } from "react";
import { toast } from "sonner";
import { PolarityIcon } from "@/components/game-icon";
import { Button } from "@/components/ui/button";
import { CheckboxField } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { api, reportError } from "@/lib/bridge";
import type { ListingChoices, RivenRow } from "@/types";
import { MAX_RIVEN_RANK } from "./columns";

interface ListingForm {
  direct: boolean;
  private: boolean;
  maxRankStats: boolean;
  sellingPrice: string;
  startingPrice: string;
  buyoutPrice: string;
  minReputation: string;
  note: string;
}

const EMPTY_LISTING: ListingForm = {
  direct: true,
  private: false,
  maxRankStats: false,
  sellingPrice: "0",
  startingPrice: "0",
  buyoutPrice: "0",
  minReputation: "0",
  note: "",
};

function wholeNumber(value: string): number {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 0;
}

function listingChoices(form: ListingForm): ListingChoices {
  return {
    direct: form.direct,
    private: form.private,
    maxRankStats: form.maxRankStats,
    sellingPrice: wholeNumber(form.sellingPrice),
    startingPrice: wholeNumber(form.startingPrice),
    buyoutPrice: wholeNumber(form.buyoutPrice),
    minReputation: wholeNumber(form.minReputation),
    note: form.note.trim() || null,
  };
}

function listingProblem(choices: ListingChoices): string | null {
  if (choices.direct) {
    return choices.sellingPrice > 0 ? null : "A selling price is required";
  }
  if (choices.startingPrice <= 0) {
    return "A starting price is required";
  }
  if (choices.buyoutPrice > 0 && choices.buyoutPrice < choices.startingPrice) {
    return "The buyout has to be at least the starting price";
  }
  return null;
}

function PriceField({
  id,
  label,
  value,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="flex flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        inputMode="numeric"
      />
    </div>
  );
}

export function RivenListingDialog({
  riven,
  onClose,
}: {
  riven: RivenRow;
  onClose: () => void;
}) {
  const [form, setForm] = useState<ListingForm>(EMPTY_LISTING);
  const [posting, setPosting] = useState(false);

  const handleSubmit = useCallback(async () => {
    const choices = listingChoices(form);
    const problem = listingProblem(choices);
    if (problem) {
      toast.error(problem);
      return;
    }
    setPosting(true);
    try {
      await api.marketPostRiven(riven.item_id, choices);
      toast.success("Listed on warframe.market");
    } catch (error) {
      reportError(error);
    } finally {
      setPosting(false);
      onClose();
    }
  }, [form, riven.item_id, onClose]);

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>List on warframe.market</DialogTitle>
          <DialogDescription className="flex items-center gap-2">
            <span>
              {`${riven.weapon ?? "Riven"} ${riven.name ?? ""}, ${riven.rerolls} rolls`}
            </span>
            {riven.polarity && (
              <PolarityIcon polarity={riven.polarity} size={20} />
            )}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="listType">Sale</Label>
              <Select
                value={form.direct ? "direct" : "auction"}
                onValueChange={(value) =>
                  setForm({ ...form, direct: value === "direct" })
                }
              >
                <SelectTrigger id="listType">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="direct">Direct sale</SelectItem>
                  <SelectItem value="auction">Auction</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="visibility">Visibility</Label>
              <Select
                value={form.private ? "private" : "public"}
                onValueChange={(value) =>
                  setForm({ ...form, private: value === "private" })
                }
              >
                <SelectTrigger id="visibility">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="public">Public</SelectItem>
                  <SelectItem value="private">Private</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {form.direct ? (
            <PriceField
              id="selling"
              label="Selling price"
              value={form.sellingPrice}
              onChange={(sellingPrice) => setForm({ ...form, sellingPrice })}
            />
          ) : (
            <div className="grid grid-cols-3 gap-4">
              <PriceField
                id="starting"
                label="Starting price"
                value={form.startingPrice}
                onChange={(startingPrice) =>
                  setForm({ ...form, startingPrice })
                }
              />
              <PriceField
                id="buyout"
                label="Buyout"
                value={form.buyoutPrice}
                onChange={(buyoutPrice) => setForm({ ...form, buyoutPrice })}
              />
              <PriceField
                id="reputation"
                label="Minimum reputation"
                value={form.minReputation}
                onChange={(minReputation) =>
                  setForm({ ...form, minReputation })
                }
              />
            </div>
          )}

          <CheckboxField
            id="maxRankStats"
            checked={form.maxRankStats}
            onChange={(maxRankStats) => setForm({ ...form, maxRankStats })}
          >
            List the stats at rank 8
            {riven.rank < MAX_RIVEN_RANK
              ? ` instead of rank ${riven.rank}`
              : ""}
          </CheckboxField>

          <div className="flex flex-col gap-2">
            <Label htmlFor="note">Note</Label>
            <Textarea
              id="note"
              value={form.note}
              onChange={(e) => setForm({ ...form, note: e.target.value })}
              placeholder="Optional note shown on the auction"
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={posting}>
            {form.direct ? "List for Sale" : "Post Auction"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
