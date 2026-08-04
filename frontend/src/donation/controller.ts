// 本文件拥有捐赠自动提示的累计计数、里程碑队列和用户选择；不拥有 Dialog、二维码或业务页面状态。
import { donationEnabled } from "./config";
import {
  applyAppOpen,
  applyAppOpens,
  applyItemsCreated,
  disableDonationAutoPrompt,
  findDonationPrompt,
  markDonationPromptShown,
  snoozeDonationPrompt,
  type DonationMilestone,
  type DonationPromptDecision,
  type DonationPromptState,
} from "./model";
import {
  browserDonationStorage,
  hasRecordedAppOpenThisSession,
  markAppOpenRecordedThisSession,
  type DonationStorage,
} from "./storage";
import type { DonationStartupTestParams } from "./testing";

export type DonationPromptListener = (decision: DonationPromptDecision) => void;

export class DonationController {
  private state: DonationPromptState;
  private pendingMilestone: DonationMilestone | null = null;
  private appOpenRecordedInRuntime = false;
  private readonly listeners = new Set<DonationPromptListener>();
  private readonly enabled: boolean;
  private readonly storage: DonationStorage;

  constructor(enabled: boolean, storage: DonationStorage = browserDonationStorage) {
    this.enabled = enabled;
    this.storage = storage;
    this.state = storage.read();
  }

  get isEnabled(): boolean {
    return this.enabled;
  }

  subscribe(listener: DonationPromptListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  recordAppOpenOnce(): void {
    if (!this.enabled || this.appOpenRecordedInRuntime || hasRecordedAppOpenThisSession()) return;
    this.appOpenRecordedInRuntime = true;
    markAppOpenRecordedThisSession();
    this.recordActivity((state, now) => applyAppOpen(state, now), true);
  }

  recordItemsCreated(count: number): void {
    if (!this.enabled) return;
    this.recordActivity((state, now) => applyItemsCreated(state, count, now), false);
  }

  recordTestStartup(params: DonationStartupTestParams): void {
    if (!this.enabled) return;
    this.recordActivity(
      (state, now) =>
        applyItemsCreated(
          applyAppOpens(state, params.additionalAppOpens, now),
          params.itemsCreated,
          now,
        ),
      false,
    );
  }

  notifyPendingPrompt(now = new Date()): void {
    if (!this.enabled) return;
    this.publishPrompt(findDonationPrompt(this.storage.read(), now, this.enabled));
  }

  evaluate(now = new Date()): DonationPromptDecision | null {
    const latest = this.storage.read();
    this.state = latest;
    return findDonationPrompt(latest, now, this.enabled);
  }

  markPromptShown(milestone: DonationMilestone, now = new Date()): void {
    if (!this.enabled) return;
    this.state = markDonationPromptShown(this.state, milestone, now);
    this.pendingMilestone = null;
    this.storage.write(this.state);
  }

  snooze(days: number, now = new Date()): void {
    if (!this.enabled) return;
    this.state = snoozeDonationPrompt(this.state, days, now);
    this.storage.write(this.state);
  }

  disableAutoPrompt(): void {
    if (!this.enabled) return;
    this.state = disableDonationAutoPrompt(this.state);
    this.storage.write(this.state);
  }

  private recordActivity(
    apply: (state: DonationPromptState, now: Date) => DonationPromptState,
    notify: boolean,
  ): void {
    const now = new Date();
    this.state = apply(this.storage.read(), now);
    this.storage.write(this.state);
    if (notify) this.publishPrompt(findDonationPrompt(this.state, now, this.enabled));
  }

  private publishPrompt(decision: DonationPromptDecision | null): void {
    if (!decision || this.pendingMilestone === decision.milestone) return;
    this.pendingMilestone = decision.milestone;
    for (const listener of this.listeners) listener(decision);
  }
}

export const donationController = new DonationController(donationEnabled);

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key === "winestock.donation-prompt.v1") donationController.evaluate();
  });
}
