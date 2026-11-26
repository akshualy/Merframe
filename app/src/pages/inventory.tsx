import { Download } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import {
  ARCANE_STATUS_LABELS,
  ArcaneCollectionView,
} from "@/components/arcane-collection-view";
import { Amount } from "@/components/currency";
import { Figure } from "@/components/figure";
import { InventoryTabView } from "@/components/inventory-tab-view";
import { OptionTabs } from "@/components/option-tabs";
import { Page, Section } from "@/components/page";
import { useSaleSafetySettings } from "@/components/sale-safety";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { VosforPackRanking } from "@/components/vosfor-pack-ranking";
import { useAsyncData } from "@/hooks/use-async-data";
import { useInventoryView } from "@/hooks/use-inventory-view";
import { useTabParam } from "@/hooks/use-tab-param";
import { api, reportError } from "@/lib/bridge";
import { num } from "@/lib/format";
import {
  ducats,
  entriesExceptSets,
  INVENTORY_TABS,
  type InventoryTabKey,
  inventoryTotals,
  listableTotals,
} from "@/lib/inventory-entries";
import { notify } from "@/lib/toast";
import { usePreferencesStore } from "@/stores/preferences-store";
import type { ArcaneCollectionStatus, ArcanesTabView } from "@/types";

const LIQUIDATION_NOTE =
  "Copies beyond what a quarter of the last 30 days of trades could absorb count at the buy price, or nothing without a buyer";

const TAB_LABELS: Record<InventoryTabKey, string> = {
  parts: "All Parts",
  relics: "Relics",
  mods: "Mods",
  arcanes: "Arcanes",
  misc: "Misc",
  sets: "Sets",
};

const ARCANE_VIEWS: { value: ArcanesTabView; label: string }[] = [
  { value: "inventory", label: "Inventory" },
  { value: "collection", label: "Collection" },
  { value: "packs", label: "Vosfor Packs" },
];

export function InventoryPage() {
  const saleSafety = useSaleSafetySettings();
  const load = useCallback(() => api.inventoryTab(saleSafety), [saleSafety]);
  const { data, error, loading } = useAsyncData(
    `inventory_tab:${JSON.stringify(saleSafety)}`,
    load,
    ["inventoryUpdated", "pricesUpdated", "favouriteUpdated", "marketUpdated"],
  );
  const [exporting, setExporting] = useState(false);
  const [tab, setTab] = useTabParam<InventoryTabKey>("parts");
  const arcaneView = usePreferencesStore((state) => state.arcaneView);
  const setArcaneView = usePreferencesStore((state) => state.setArcaneView);

  const { entries, flags, filters, updateFilters, visible, listable } =
    useInventoryView(tab, data);
  const pageTotals = useMemo(
    () => (listable ? listableTotals(visible) : inventoryTotals(visible)),
    [visible, listable],
  );
  const pageScope = listable
    ? "Copies safe to list in the rows shown"
    : "Rows shown";
  const accountValue = useMemo(
    () =>
      data ? inventoryTotals(entriesExceptSets(data)).liquidationValue : 0,
    [data],
  );
  const arcaneData =
    tab === "arcanes" && arcaneView !== "inventory" ? data : null;
  const collected = useMemo(() => {
    const rows = data ? data.arcane_collection : [];
    const withStatus = (status: ArcaneCollectionStatus) =>
      rows.filter((row) => row.status === status).length;
    return {
      total: rows.length,
      maxed: withStatus("maxed"),
      canBeMaxed: withStatus("can_be_maxed"),
    };
  }, [data]);
  const showsDucats =
    tab !== "sets" && entries.some((entry) => ducats(entry) !== null);

  const handleExport = useCallback(async () => {
    setExporting(true);
    try {
      const dir = await api.exportBundle();
      if (dir) {
        notify.success("Exported", { description: dir });
      }
    } catch (error) {
      reportError(error);
    } finally {
      setExporting(false);
    }
  }, []);

  return (
    <Page
      title="Inventory"
      quote="inventory"
      actions={
        <Button variant="outline" onClick={handleExport} pending={exporting}>
          <Download className="size-4" />
          Export JSON
        </Button>
      }
      loading={loading && !data}
      error={error}
    >
      <Section>
        <Tabs value={tab} onValueChange={setTab}>
          <div className="mb-4 flex flex-wrap items-end justify-between gap-4">
            <TabsList>
              {INVENTORY_TABS.map((key) => (
                <TabsTrigger key={key} value={key}>
                  {TAB_LABELS[key]} ({num(data?.[key].length ?? 0)})
                </TabsTrigger>
              ))}
            </TabsList>
            <div className="flex flex-wrap items-center gap-x-6 gap-y-1">
              {arcaneData ? (
                <>
                  <Figure
                    label={ARCANE_STATUS_LABELS.maxed}
                    value={`${num(collected.maxed)} of ${num(collected.total)}`}
                  />
                  <Figure
                    label={ARCANE_STATUS_LABELS.can_be_maxed}
                    title="Arcanes with enough copies to reach their max rank"
                    value={num(collected.canBeMaxed)}
                  />
                </>
              ) : (
                <Figure
                  label="This tab"
                  title={`${pageScope}. ${LIQUIDATION_NOTE}`}
                  value={
                    <span className="flex items-center gap-3">
                      <Amount currency="platinum">
                        {num(pageTotals.liquidationValue)}
                      </Amount>
                      {showsDucats && (
                        <Amount currency="ducats">
                          {num(pageTotals.ducats)}
                        </Amount>
                      )}
                    </span>
                  }
                />
              )}
              <Figure
                label="Account"
                title={`Every tab but Sets, which holds the same parts. ${LIQUIDATION_NOTE}`}
                value={<Amount currency="platinum">{num(accountValue)}</Amount>}
              />
            </div>
          </div>

          <TabsContent value={tab} className="flex flex-col gap-3">
            {tab === "arcanes" && (
              <OptionTabs
                value={arcaneView}
                options={ARCANE_VIEWS}
                onChange={setArcaneView}
              />
            )}
            {arcaneData === null ? (
              <InventoryTabView
                key={tab}
                tab={tab}
                entryCount={entries.length}
                visible={visible}
                unpriced={pageTotals.unpriced}
                flags={flags}
                filters={filters}
                updateFilters={updateFilters}
              />
            ) : arcaneView === "packs" ? (
              <VosforPackRanking data={arcaneData} />
            ) : (
              <ArcaneCollectionView data={arcaneData} />
            )}
          </TabsContent>
        </Tabs>
      </Section>
    </Page>
  );
}
