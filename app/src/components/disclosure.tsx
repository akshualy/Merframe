import { exit } from "@tauri-apps/plugin-process";
import { useCallback } from "react";
import { BrandBar } from "@/components/brand-bar";
import {
  ExternalLinkButton,
  type ExternalLinkTarget,
} from "@/components/external-link-button";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { reportError } from "@/lib/bridge";
import { useAppStore } from "@/stores/app-store";

export const TERMS_LINK: ExternalLinkTarget = {
  label: "Terms of Use",
  url: "https://yareli.net/terms",
};

const DISCLOSURE_LINKS: ExternalLinkTarget[] = [
  TERMS_LINK,
  {
    label: "Legality & Risks",
    url: "https://github.com/akshualy/Merframe/wiki/Legality-&-Risks",
  },
  {
    label: "Third Parties",
    url: "https://github.com/akshualy/Merframe#privacy",
  },
];

export function Disclosure() {
  return (
    <div className="text-muted-foreground flex flex-col gap-3 text-sm">
      <p>
        <span className="text-foreground">Merframe</span> reads{" "}
        <span className="text-foreground">game memory</span> and the{" "}
        <span className="text-foreground">EE.log</span> file Warframe writes to
        your file system.
      </p>
      <p>
        This does not put Merframe in a unique position,{" "}
        <span className="text-foreground font-bold">
          the same rules apply to every Warframe third-party app
        </span>
        .
      </p>
      <p>
        Merframe does not write to or inject into Warframe's process, automate
        input, or make calls to DE's API outside of the public worldState. We do
        not upload game or account data to third parties.
      </p>
      <p>
        Memory contains more information than what the game shows to users.
        Merframe only displays what a player could also work out by hand from
        what the game shows them. If it can be done with pen and paper, we may
        show it.
      </p>
      <p>
        DE's publicly stated rule is{" "}
        <span className="text-foreground">
          &quot;If you use external software in conjunction with Warframe, then
          you do so at your own risk&quot;
        </span>
        . Merframe is a fan project and is not affiliated with Digital Extremes.
      </p>
      <p>If you are not comfortable with this, you may close the application at any point.</p>
    </div>
  );
}

export function DisclosureScreen() {
  const saveSettings = useAppStore((state) => state.saveSettings);
  const handleAccept = useCallback(
    () => saveSettings({ first_start_disclosure: "accepted" }),
    [saveSettings],
  );

  const handleQuit = useCallback(() => exit(0).catch(reportError), []);

  return (
    <div className="bg-background flex h-full flex-col">
      <BrandBar />

      <main className="flex min-h-0 flex-1 justify-center overflow-y-auto p-8">
        <div className="flex w-full max-w-xl flex-col gap-6">
          <h1 className="text-primary text-2xl font-bold">Before You Start</h1>

          <div className="flex flex-col gap-3">
            <Disclosure />
            <div className="flex flex-wrap gap-2">
              {DISCLOSURE_LINKS.map((link) => (
                <ExternalLinkButton key={link.url} url={link.url}>
                  {link.label}
                </ExternalLinkButton>
              ))}
            </div>
          </div>

          <div className="flex flex-col gap-3 border-t pt-4">
            <Hint>
              Merframe only reads game data once you accept. The above text is
              always available on the About page.
            </Hint>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={handleQuit}>
                Quit
              </Button>
              <Button onClick={handleAccept}>Accept</Button>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
