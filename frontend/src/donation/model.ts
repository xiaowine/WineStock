// 本文件拥有捐赠累计计数、里程碑和冷却状态的纯逻辑；不依赖 Vue、浏览器存储或具体 UI。

export const DONATION_STORAGE_KEY = "winestock.donation-prompt.v1";
export const DONATION_SESSION_OPEN_KEY = "winestock.donation-open-recorded.v1";

export type DonationMilestone =
  "app-open-10" | "app-open-50" | "items-created-50" | "items-created-100" | "items-created-300";

export type DonationPromptReason = "app-open" | "items-created";

export interface DonationPromptState {
  version: 1;
  totalAppOpens: number;
  totalItemsCreated: number;
  reachedMilestonesAt: Partial<Record<DonationMilestone, string>>;
  acknowledgedMilestones: DonationMilestone[];
  promptCount: number;
  lastPromptAt: string | null;
  snoozeUntil: string | null;
  autoPromptDisabled: boolean;
}

export interface DonationPromptDecision {
  milestone: DonationMilestone;
  reason: DonationPromptReason;
}

interface MilestoneDefinition {
  id: DonationMilestone;
  reason: DonationPromptReason;
  threshold: number;
}

export const DONATION_MILESTONES: readonly MilestoneDefinition[] = [
  { id: "app-open-10", reason: "app-open", threshold: 10 },
  { id: "app-open-50", reason: "app-open", threshold: 50 },
  { id: "items-created-50", reason: "items-created", threshold: 50 },
  { id: "items-created-100", reason: "items-created", threshold: 100 },
  { id: "items-created-300", reason: "items-created", threshold: 300 },
];

const MILESTONE_IDS = new Set<DonationMilestone>(DONATION_MILESTONES.map(({ id }) => id));

export function createDefaultDonationPromptState(): DonationPromptState {
  return {
    version: 1,
    totalAppOpens: 0,
    totalItemsCreated: 0,
    reachedMilestonesAt: {},
    acknowledgedMilestones: [],
    promptCount: 0,
    lastPromptAt: null,
    snoozeUntil: null,
    autoPromptDisabled: false,
  };
}

export function normalizeDonationPromptState(value: unknown): DonationPromptState {
  if (!isRecord(value) || value.version !== 1) return createDefaultDonationPromptState();

  const reachedMilestonesAt: Partial<Record<DonationMilestone, string>> = {};
  if (isRecord(value.reachedMilestonesAt)) {
    for (const id of DONATION_MILESTONES.map(({ id }) => id)) {
      const timestamp = value.reachedMilestonesAt[id];
      if (isValidTimestamp(timestamp)) reachedMilestonesAt[id] = timestamp;
    }
  }

  const acknowledgedMilestones = Array.isArray(value.acknowledgedMilestones)
    ? value.acknowledgedMilestones.filter(isDonationMilestone)
    : [];

  return {
    version: 1,
    totalAppOpens: nonNegativeInteger(value.totalAppOpens),
    totalItemsCreated: nonNegativeInteger(value.totalItemsCreated),
    reachedMilestonesAt,
    acknowledgedMilestones: [...new Set(acknowledgedMilestones)],
    promptCount: nonNegativeInteger(value.promptCount),
    lastPromptAt: isValidTimestamp(value.lastPromptAt) ? value.lastPromptAt : null,
    snoozeUntil: isValidTimestamp(value.snoozeUntil) ? value.snoozeUntil : null,
    autoPromptDisabled: value.autoPromptDisabled === true,
  };
}

export function applyAppOpen(state: DonationPromptState, now: Date): DonationPromptState {
  return applyAppOpens(state, 1, now);
}

export function applyAppOpens(
  state: DonationPromptState,
  count: number,
  now: Date,
): DonationPromptState {
  if (!Number.isSafeInteger(count) || count <= 0) return state;
  const totalAppOpens = state.totalAppOpens + count;
  if (!Number.isSafeInteger(totalAppOpens)) return state;
  return applyActivity({ ...state, totalAppOpens }, now);
}

export function applyItemsCreated(
  state: DonationPromptState,
  count: number,
  now: Date,
): DonationPromptState {
  if (!Number.isSafeInteger(count) || count <= 0) return state;
  const totalItemsCreated = state.totalItemsCreated + count;
  if (!Number.isSafeInteger(totalItemsCreated)) return state;
  return applyActivity({ ...state, totalItemsCreated }, now);
}

export function findDonationPrompt(
  state: DonationPromptState,
  now: Date,
  enabled: boolean,
): DonationPromptDecision | null {
  if (!enabled || state.autoPromptDisabled) return null;
  if (state.snoozeUntil && Date.parse(state.snoozeUntil) > now.getTime()) return null;

  const acknowledged = new Set(state.acknowledgedMilestones);
  const pending = DONATION_MILESTONES.map((definition, order) => ({
    ...definition,
    order,
    reachedAt: state.reachedMilestonesAt[definition.id],
  }))
    .filter(({ id, reachedAt }) => Boolean(reachedAt) && !acknowledged.has(id))
    .sort((left, right) => {
      const byTime = Date.parse(left.reachedAt!) - Date.parse(right.reachedAt!);
      return byTime || left.order - right.order;
    });

  const first = pending[0];
  return first ? { milestone: first.id, reason: first.reason } : null;
}

export function markDonationPromptShown(
  state: DonationPromptState,
  milestone: DonationMilestone,
  now: Date,
): DonationPromptState {
  if (!MILESTONE_IDS.has(milestone)) return state;
  return {
    ...state,
    acknowledgedMilestones: [...new Set([...state.acknowledgedMilestones, milestone])],
    promptCount: state.promptCount + 1,
    lastPromptAt: now.toISOString(),
  };
}

export function snoozeDonationPrompt(
  state: DonationPromptState,
  days: number,
  now: Date,
): DonationPromptState {
  const duration = Math.max(0, days) * 24 * 60 * 60 * 1000;
  const nextUntil = new Date(now.getTime() + duration).toISOString();
  const existingUntil = state.snoozeUntil && Date.parse(state.snoozeUntil);
  return {
    ...state,
    snoozeUntil:
      existingUntil && Number.isFinite(existingUntil) && existingUntil > Date.parse(nextUntil)
        ? state.snoozeUntil
        : nextUntil,
  };
}

export function disableDonationAutoPrompt(state: DonationPromptState): DonationPromptState {
  return { ...state, autoPromptDisabled: true };
}

function applyActivity(state: DonationPromptState, now: Date): DonationPromptState {
  const reachedMilestonesAt = { ...state.reachedMilestonesAt };
  for (const definition of DONATION_MILESTONES) {
    const total = definition.reason === "app-open" ? state.totalAppOpens : state.totalItemsCreated;
    if (total >= definition.threshold && !reachedMilestonesAt[definition.id]) {
      reachedMilestonesAt[definition.id] = now.toISOString();
    }
  }
  return { ...state, reachedMilestonesAt };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function nonNegativeInteger(value: unknown): number {
  return Number.isSafeInteger(value) && (value as number) >= 0 ? (value as number) : 0;
}

function isValidTimestamp(value: unknown): value is string {
  return typeof value === "string" && Number.isFinite(Date.parse(value));
}

function isDonationMilestone(value: unknown): value is DonationMilestone {
  return typeof value === "string" && MILESTONE_IDS.has(value as DonationMilestone);
}
