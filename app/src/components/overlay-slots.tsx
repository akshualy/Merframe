import { EmptyNote } from "@/components/page";
import {
  overlaySquadSize,
  RelicRecommendationOverlay,
} from "@/components/relic-recommendation-overlay";
import { RelicRewardOverlay } from "@/components/relic-reward-overlay";
import { RivenOverlay, RivenRerollOverlay } from "@/components/riven-overlay";
import { useLoaded } from "@/hooks/use-loaded";
import { api, logError } from "@/lib/bridge";
import type {
  RecommendationTrigger,
  RewardScreen,
  RewardTrigger,
  RivenTrigger,
} from "@/types";

const EMPTY_SCREEN: RewardScreen = {
  ranked: [],
  account: null,
};

export function RewardSlot({
  trigger,
  compact = false,
}: {
  trigger: RewardTrigger | null;
  compact?: boolean;
}) {
  const screen = useLoaded(
    trigger,
    (open) => api.recommend(open.rewards),
    (cause) => {
      logError("Ranking the relic rewards", cause);
      return EMPTY_SCREEN;
    },
  );

  if (!trigger) {
    return <EmptyNote>Open a relic to see squad rewards and prices.</EmptyNote>;
  }
  if (screen === null) {
    return <EmptyNote>Pricing the rewards.</EmptyNote>;
  }
  return <RelicRewardOverlay screen={screen} compact={compact} />;
}

export function RecommendationSlot({
  trigger,
  compact = false,
}: {
  trigger: RecommendationTrigger | null;
  compact?: boolean;
}) {
  const plans = useLoaded(
    trigger,
    async () => (await api.relicPlannerTab(overlaySquadSize())).plans,
    (cause) => {
      logError("Ranking the owned relics", cause);
      return [];
    },
  );

  if (!trigger) {
    return <EmptyNote>Choose a fissure to rank your relics for it.</EmptyNote>;
  }
  if (plans === null) {
    return <EmptyNote>Reading the owned relics.</EmptyNote>;
  }
  return (
    <RelicRecommendationOverlay
      tier={trigger.tier}
      plans={plans}
      refinement={trigger.refinement}
      limit={trigger.count}
      compact={compact}
    />
  );
}

export function RivenSlot({
  trigger,
  compact = false,
}: {
  trigger: RivenTrigger | null;
  compact?: boolean;
}) {
  const attribution = useLoaded(
    trigger,
    async () => (await api.rivensTab()).attribution,
    (cause) => {
      logError("Reading the good roll attribution", cause);
      return null;
    },
  );

  if (!trigger) {
    return (
      <EmptyNote>
        Open a riven or the reroll station to compare rolls.
      </EmptyNote>
    );
  }
  const { before, linked } = trigger;
  if (before) {
    const after = before.pending
      ? { ...before, ...before.pending, pending: null }
      : null;
    return (
      <RivenRerollOverlay
        before={before}
        after={after}
        attribution={attribution}
        compact={compact}
      />
    );
  }
  if (!linked) {
    return (
      <EmptyNote>
        The riven in the dialog could not be read from the game.
      </EmptyNote>
    );
  }
  return (
    <RivenOverlay
      rivens={[linked]}
      attribution={attribution}
      compact={compact}
    />
  );
}
