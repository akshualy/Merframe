import { Download } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useSearchParams } from "react-router";
import { InventoryTabView } from "@/components/inventory-tab-view";
import {
  ErrorNote,
  Page,
  Quoted,
  Section,
  TableSkeleton,
} from "@/components/page";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAsyncData } from "@/hooks/use-async-data";
import { api, events, reportError } from "@/lib/bridge";
import { num } from "@/lib/format";
import { entriesFor, inventoryFlags } from "@/lib/inventory-entries";
import type { InventoryTabKey } from "@/lib/inventory-filters";
import { usePageQuote } from "@/lib/quotes";
import { notify } from "@/lib/toast";

export function InventoryPage() {
  const quote = usePageQuote("inventory");
  const { data, error, loading } = useAsyncData(api.inventoryTab, [
    events.inventoryUpdated,
    events.pricesUpdated,
    events.favouriteUpdated,
    events.marketUpdated,
  ]);
  const [exporting, setExporting] = useState(false);
  const [params, setParams] = useSearchParams();
  const tab = (params.get("tab") as InventoryTabKey | null) ?? "parts";

  const entries = useMemo(() => entriesFor(tab, data), [tab, data]);
  const flags = useMemo(() => inventoryFlags(data), [data]);

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

  if (loading && !data) {
    return (
      <Page title="Inventory" description={<Quoted quote={quote} />}>
        <TableSkeleton />
      </Page>
    );
  }

  return (
    <Page
      title="Inventory"
      description={<Quoted quote={quote} />}
      actions={
        <Button variant="outline" onClick={handleExport} disabled={exporting}>
          <Download className="size-4" />
          Export JSON
        </Button>
      }
    >
      {error && <ErrorNote message={error} />}

      <Section>
        <Tabs
          value={tab}
          onValueChange={(value) =>
            setParams({ tab: value }, { replace: true })
          }
        >
          <TabsList className="mb-4">
            <TabsTrigger value="parts">
              All Parts ({num(data?.parts.length ?? 0)})
            </TabsTrigger>
            <TabsTrigger value="relics">
              Relics ({num(data?.relics.length ?? 0)})
            </TabsTrigger>
            <TabsTrigger value="mods">
              Mods ({num(data?.mods.length ?? 0)})
            </TabsTrigger>
            <TabsTrigger value="arcanes">
              Arcanes ({num(data?.arcanes.length ?? 0)})
            </TabsTrigger>
            <TabsTrigger value="misc">
              Misc ({num(data?.misc.length ?? 0)})
            </TabsTrigger>
            <TabsTrigger value="sets">
              Sets ({num(data?.sets.length ?? 0)})
            </TabsTrigger>
          </TabsList>

          <TabsContent value={tab}>
            <InventoryTabView
              key={tab}
              tab={tab}
              entries={entries}
              flags={flags}
            />
          </TabsContent>
        </Tabs>
      </Section>
    </Page>
  );
}
