// 本文件拥有捐赠构建配置与可用性判断，属于 frontend 捐赠领域；不读取运行时业务数据，也不保存秘密。

export type DonationMethodId = "wechat" | "alipay";

export interface DonationMethod {
  id: DonationMethodId;
  label: string;
  content: string;
}

function readEnv(value: string | undefined): string {
  return value?.trim() ?? "";
}

export const donationConfig = {
  wechatContent: readEnv(import.meta.env.VITE_DONATION_WECHAT_CONTENT),
  alipayContent: readEnv(import.meta.env.VITE_DONATION_ALIPAY_CONTENT),
} as const;

export const donationMethods: readonly DonationMethod[] = [
  donationConfig.wechatContent
    ? { id: "wechat", label: "微信", content: donationConfig.wechatContent }
    : null,
  donationConfig.alipayContent
    ? { id: "alipay", label: "支付宝", content: donationConfig.alipayContent }
    : null,
].filter((method): method is DonationMethod => method !== null);

/** 没有任何公开捐赠内容时，入口和自动提示都必须关闭。 */
export const donationEnabled = donationMethods.length > 0;
