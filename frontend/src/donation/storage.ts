// 本文件拥有捐赠控制器的版本化本地存储适配，属于 frontend 捐赠领域；存储不可用时不阻塞业务。
import {
  createDefaultDonationPromptState,
  DONATION_SESSION_OPEN_KEY,
  DONATION_STORAGE_KEY,
  normalizeDonationPromptState,
  type DonationPromptState,
} from "./model";

export interface DonationStorage {
  read(): DonationPromptState;
  write(state: DonationPromptState): void;
}

export const browserDonationStorage: DonationStorage = {
  read() {
    try {
      const raw = window.localStorage.getItem(DONATION_STORAGE_KEY);
      return raw
        ? normalizeDonationPromptState(JSON.parse(raw))
        : createDefaultDonationPromptState();
    } catch {
      return createDefaultDonationPromptState();
    }
  },
  write(state) {
    try {
      window.localStorage.setItem(DONATION_STORAGE_KEY, JSON.stringify(state));
    } catch {
      // 本次会话继续使用内存状态；捐赠提示不能阻塞库存业务。
    }
  },
};

export function hasRecordedAppOpenThisSession(): boolean {
  try {
    return window.sessionStorage.getItem(DONATION_SESSION_OPEN_KEY) === "1";
  } catch {
    return false;
  }
}

export function markAppOpenRecordedThisSession(): void {
  try {
    window.sessionStorage.setItem(DONATION_SESSION_OPEN_KEY, "1");
  } catch {
    // sessionStorage 不可用时由控制器的内存保护避免当前运行时重复计数。
  }
}
