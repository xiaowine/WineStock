// 本文件拥有开发环境捐赠启动测试参数的解析，不参与生产构建行为或业务计数规则。

export interface DonationStartupTestParams {
  /** 当前启动之前额外模拟的累计启动次数；当前真实启动仍由 recordAppOpenOnce 记录。 */
  additionalAppOpens: number;
  /** 当前启动时额外模拟的成功新增物品数量。 */
  itemsCreated: number;
}

export function readDonationStartupTestParams(
  search: string,
  hash = "",
): DonationStartupTestParams | null {
  const params = new URLSearchParams(search);
  const hashQueryIndex = hash.indexOf("?");
  if (hashQueryIndex >= 0) {
    const hashParams = new URLSearchParams(hash.slice(hashQueryIndex + 1));
    for (const [key, value] of hashParams) {
      if (!params.has(key)) params.set(key, value);
    }
  }

  const additionalAppOpens = readCount(params.get("donation_test_opens"));
  const itemsCreated = readCount(params.get("donation_test_items"));
  if (additionalAppOpens === 0 && itemsCreated === 0) return null;
  return { additionalAppOpens, itemsCreated };
}

function readCount(value: string | null): number {
  if (!value || !/^\d+$/.test(value)) return 0;
  const count = Number(value);
  return Number.isSafeInteger(count) ? count : 0;
}
