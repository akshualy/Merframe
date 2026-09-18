import { getVersion } from "@tauri-apps/api/app";
import { ExternalLink } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Merframe from "@/components/icons/merframe";
import { Page, Quoted, Section } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { api, reportError } from "@/lib/bridge";
import { ago } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import { useAppStore } from "@/stores/app-store";

const LINKS = [
  { label: "warframe.market", url: "https://warframe.market" },
  {
    label: "WFCD warframe-items",
    url: "https://github.com/WFCD/warframe-items",
  },
  { label: "Warframe wiki", url: "https://wiki.warframe.com" },
];

export function AboutPage() {
  const quote = usePageQuote("about");
  const { status } = useAppStore();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion, reportError);
  }, []);

  const handleOpen = useCallback(async (url: string) => {
    try {
      await api.openUrl(url);
    } catch (error) {
      reportError(error);
    }
  }, []);

  return (
    <Page title="About" description={<Quoted quote={quote} />}>
      <Section>
        <div className="flex flex-col gap-6 sm:flex-row sm:items-center">
          <Merframe className="text-primary size-24 shrink-0" />
          <div className="flex flex-col gap-3 text-sm">
            <p className="text-muted-foreground max-w-2xl">
              Merframe is a re-creation of AlecaFrame for Linux and Windows. It
              reads the running game and EE.log. Nothing is uploaded, snapshots
              stay in a local SQLite database.
            </p>
            <div className="flex flex-wrap gap-2">
              <Badge variant="secondary">Version {version}</Badge>
              <Badge variant="secondary">Tauri 2</Badge>
              <Badge variant="secondary">React 19</Badge>
              <Badge variant="secondary">Rust 2024</Badge>
            </div>
          </div>
        </div>
      </Section>

      <Section title="This Machine">
        <dl className="grid gap-3 sm:grid-cols-2">
          <div className="flex flex-col">
            <Hint as="dt">Game process</Hint>
            <dd className="text-sm">
              {status?.game_detected
                ? `running (pid ${status.pid ?? "?"})`
                : "not running"}
            </dd>
          </div>
          <div className="flex flex-col">
            <Hint as="dt">Last scan</Hint>
            <dd className="text-sm">{ago(status?.last_scan_at)}</dd>
          </div>
          <div className="flex flex-col">
            <Hint as="dt">World state</Hint>
            <dd className="text-sm">{ago(status?.world_state_at)}</dd>
          </div>
          <div className="flex flex-col sm:col-span-2">
            <Hint as="dt">EE.log</Hint>
            <dd className="text-sm break-all">
              {status?.log_file ?? "not found"}
            </dd>
          </div>
        </dl>
      </Section>

      <Section title="Sources">
        <div className="flex flex-wrap gap-2">
          {LINKS.map((link) => (
            <Button
              key={link.url}
              variant="outline"
              size="sm"
              onClick={() => handleOpen(link.url)}
            >
              {link.label}
              <ExternalLink className="size-3.5" />
            </Button>
          ))}
        </div>
      </Section>
    </Page>
  );
}
