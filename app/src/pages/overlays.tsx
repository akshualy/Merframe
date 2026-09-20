import { type ReactNode, useEffect, useRef, useState } from "react";
import {
  RecommendationSlot,
  RewardSlot,
  RivenSlot,
} from "@/components/overlay-slots";
import { Page, Quoted, Section } from "@/components/page";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { api, reportError } from "@/lib/bridge";
import { usePageQuote } from "@/lib/quotes";
import { useAppStore } from "@/stores/app-store";
import { useOverlayStore } from "@/stores/overlay-store";
import type {
  OverlayMode,
  OverlayPlacement,
  OverlaySupport,
  RecommendationRefinement,
  Settings,
} from "@/types";

const OFF = "Turned off.";

const OVERLAY_MODES: { value: OverlayMode; label: string }[] = [
  { value: "auto", label: "Windows when the session allows it" },
  { value: "windows", label: "Always windows over the game" },
  { value: "tab", label: "Only the Overlays tab" },
];

const PLACEMENTS: { value: OverlayPlacement; label: string }[] = [
  { value: "top_left", label: "Top left" },
  { value: "top_right", label: "Top right" },
  { value: "bottom_left", label: "Bottom left" },
  { value: "bottom_right", label: "Bottom right" },
  { value: "centre", label: "Centre" },
];

const REFINEMENTS: { value: RecommendationRefinement; label: string }[] = [
  { value: "radiant", label: "Always Radiant" },
  { value: "owned", label: "Owned refinement" },
];

function Field({
  id,
  label,
  children,
}: {
  id: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center gap-2">
      <Label htmlFor={id} className="text-sm">
        {label}
      </Label>
      {children}
    </div>
  );
}

function renderNote(mode: OverlayMode, support: OverlaySupport | undefined) {
  if (mode === "tab") {
    return "Overlays render in this tab only.";
  }
  if (mode === "windows") {
    return "Overlays render as windows over the game and here.";
  }
  return support?.windows_possible
    ? "Overlays render as windows over the game and here."
    : "Overlay windows are unavailable in this session. Overlays render in this tab only.";
}

function Focused({
  trigger,
  children,
}: {
  trigger: unknown;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const node = ref.current;
    if (!trigger || !node) {
      return;
    }
    const reveal = () =>
      node.scrollIntoView({ behavior: "smooth", block: "nearest" });
    reveal();
    const observer = new ResizeObserver(reveal);
    observer.observe(node);
    return () => observer.disconnect();
  }, [trigger]);

  return <div ref={ref}>{children}</div>;
}

function CardControls({
  name,
  enabled,
  placement,
  disabled,
  onEnabled,
  onPlacement,
  children,
}: {
  name: string;
  enabled: boolean;
  placement: OverlayPlacement;
  disabled: boolean;
  onEnabled: (enabled: boolean) => void;
  onPlacement: (placement: OverlayPlacement) => void;
  children?: ReactNode;
}) {
  return (
    <div className="mb-4 flex flex-wrap items-center gap-x-6 gap-y-2">
      <Field id={`${name}-enabled`} label="Enabled">
        <Switch
          id={`${name}-enabled`}
          checked={enabled}
          disabled={disabled}
          onCheckedChange={onEnabled}
        />
      </Field>
      <Field id={`${name}-placement`} label="Placement">
        <Select
          value={placement}
          disabled={disabled}
          onValueChange={(value) => onPlacement(value as OverlayPlacement)}
        >
          <SelectTrigger id={`${name}-placement`} size="sm" className="w-36">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PLACEMENTS.map(({ value, label }) => (
              <SelectItem key={value} value={value}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      {children}
    </div>
  );
}

export function OverlaysPage() {
  const quote = usePageQuote("overlays");
  const { status, settings, setSettings } = useAppStore();
  const { overlays } = useOverlayStore();
  const [dragged, setDragged] = useState<number | null>(null);
  const [draggedCount, setDraggedCount] = useState<number | null>(null);

  if (!settings) {
    return null;
  }

  const update = async (next: Partial<Settings>) => {
    const merged = { ...settings, ...next };
    setSettings(merged);
    try {
      await api.settingsSet(merged);
    } catch (error) {
      reportError(error);
    }
  };

  const master = settings.overlays_enabled;
  const opacity = dragged ?? settings.overlay_opacity;
  const count = draggedCount ?? settings.overlay_recommendation_count;
  const reward = master && settings.overlay_relic_reward;
  const recommendation = master && settings.overlay_relic_recommendation;
  const riven = master && settings.overlay_riven;

  return (
    <Page title="Overlays" description={<Quoted quote={quote} />}>
      <Section
        title="Overlay Settings"
        description={renderNote(settings.overlay_mode, status?.overlay_support)}
        action={
          <div className="flex flex-wrap items-center gap-x-6 gap-y-2">
            <Field id="overlays-enabled" label="Enable overlays">
              <Switch
                id="overlays-enabled"
                checked={master}
                onCheckedChange={(checked) =>
                  update({ overlays_enabled: checked })
                }
              />
            </Field>
            <Field
              id="overlays-game-active"
              label="Only while the game is active"
            >
              <Switch
                id="overlays-game-active"
                checked={settings.overlay_only_while_game_active}
                disabled={!master}
                onCheckedChange={(checked) =>
                  update({ overlay_only_while_game_active: checked })
                }
              />
            </Field>
          </div>
        }
      >
        <div className="flex flex-col gap-4">
          <div className="flex flex-wrap items-center gap-x-6 gap-y-3">
            <Field id="overlay-mode" label="Where an overlay renders">
              <Select
                value={settings.overlay_mode}
                onValueChange={(value) =>
                  update({ overlay_mode: value as OverlayMode })
                }
              >
                <SelectTrigger id="overlay-mode" size="sm" className="w-64">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {OVERLAY_MODES.map(({ value, label }) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <div className="flex items-center gap-3">
              <Label htmlFor="overlay-opacity" className="text-sm">
                Overlay opacity
              </Label>
              <Slider
                id="overlay-opacity"
                className="w-40"
                min={50}
                max={100}
                step={5}
                value={[opacity]}
                disabled={!master}
                onValueChange={([value = opacity]) => setDragged(value)}
                onValueCommit={([value = opacity]) => {
                  setDragged(null);
                  update({ overlay_opacity: value });
                }}
              />
              <span className="text-muted-foreground w-10 text-sm tabular-nums">
                {opacity}%
              </span>
            </div>
          </div>
          <div className="text-muted-foreground flex flex-col gap-1 text-sm">
            <p>
              &quot;Only while the game is active&quot; hides the overlay
              windows while Warframe is not the focused window.
            </p>
            <p>
              {status?.overlay_support.detail ??
                "The overlay support of this session is not known yet."}
            </p>
          </div>
        </div>
      </Section>

      <Section
        title="Relic Rewards"
        description={
          reward ? "Shown while the reward screen is on display." : OFF
        }
      >
        <CardControls
          name="reward"
          enabled={settings.overlay_relic_reward}
          placement={settings.overlay_relic_reward_placement}
          disabled={!master}
          onEnabled={(enabled) => update({ overlay_relic_reward: enabled })}
          onPlacement={(value) =>
            update({ overlay_relic_reward_placement: value })
          }
        >
          <Field id="reward-balance" label="Show your Platinum and Ducats">
            <Switch
              id="reward-balance"
              checked={settings.overlay_account_balance}
              disabled={!master}
              onCheckedChange={(checked) =>
                update({ overlay_account_balance: checked })
              }
            />
          </Field>
          <Field id="reward-copy" label="Copy the rewards to the clipboard">
            <Switch
              id="reward-copy"
              checked={settings.copy_relic_rewards}
              onCheckedChange={(checked) =>
                update({ copy_relic_rewards: checked })
              }
            />
          </Field>
        </CardControls>
        <Focused trigger={reward ? overlays.reward : null}>
          <RewardSlot trigger={reward ? overlays.reward : null} />
        </Focused>
      </Section>

      <Section
        title="Relic Recommendation"
        description={
          recommendation
            ? "Shown while the relic select screen is on display. Always Radiant ranks every relic as if you refined it first."
            : OFF
        }
      >
        <CardControls
          name="recommendation"
          enabled={settings.overlay_relic_recommendation}
          placement={settings.overlay_relic_recommendation_placement}
          disabled={!master}
          onEnabled={(enabled) =>
            update({ overlay_relic_recommendation: enabled })
          }
          onPlacement={(value) =>
            update({ overlay_relic_recommendation_placement: value })
          }
        >
          <Field id="recommendation-refinement" label="Values">
            <Select
              value={settings.overlay_recommendation_refinement}
              disabled={!master}
              onValueChange={(value) =>
                update({
                  overlay_recommendation_refinement:
                    value as RecommendationRefinement,
                })
              }
            >
              <SelectTrigger
                id="recommendation-refinement"
                size="sm"
                className="w-48"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {REFINEMENTS.map(({ value, label }) => (
                  <SelectItem key={value} value={value}>
                    {label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <div className="flex items-center gap-3">
            <Label htmlFor="recommendation-count" className="text-sm">
              Relics shown
            </Label>
            <Slider
              id="recommendation-count"
              className="w-28"
              min={3}
              max={10}
              step={1}
              value={[count]}
              disabled={!master}
              onValueChange={([value = count]) => setDraggedCount(value)}
              onValueCommit={([value = count]) => {
                setDraggedCount(null);
                update({ overlay_recommendation_count: value });
              }}
            />
            <span className="text-muted-foreground w-5 text-sm tabular-nums">
              {count}
            </span>
          </div>
        </CardControls>
        <Focused trigger={recommendation ? overlays.recommendation : null}>
          <RecommendationSlot
            trigger={recommendation ? overlays.recommendation : null}
          />
        </Focused>
      </Section>

      <Section
        title="Riven"
        description={riven ? "Shown while you inspect or cycle a riven." : OFF}
      >
        <CardControls
          name="riven"
          enabled={settings.overlay_riven}
          placement={settings.overlay_riven_placement}
          disabled={!master}
          onEnabled={(enabled) => update({ overlay_riven: enabled })}
          onPlacement={(value) => update({ overlay_riven_placement: value })}
        />
        <Focused trigger={riven ? overlays.riven : null}>
          <RivenSlot trigger={riven ? overlays.riven : null} />
        </Focused>
      </Section>
    </Page>
  );
}
