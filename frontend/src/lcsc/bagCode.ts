// 本文件拥有立创料袋二维码文本的解析纯逻辑；它不执行扫码、网络请求或物品匹配。

/** 立创料袋二维码解析结果；字段缺失或为占位值时为 null。 */
export interface LcscBagCode {
  /** 立创订单号，如 SO26010926692。 */
  orderNo: string | null;
  /** 立创商品编号（C 号），已归一化为大写 C 前缀。 */
  productCode: string;
  /** 制造商型号。 */
  manufacturerPart: string | null;
  /** 袋内数量；缺失或非法时为 null。 */
  quantity: number | null;
}

const MAX_INPUT_LENGTH = 512;

/**
 * 解析立创料袋二维码文本，形如
 * `{on:SO26010926692,pc:C2687125,pm:SM05.TCT,qty:100,mc:,cc:1,pdi:193942893,hp:11}`。
 * 键值均不带引号，属于立创私有格式；无法确认为料袋码时返回 null，调用方按非立创内容忽略。
 */
export function parseLcscBagCode(rawText: string): LcscBagCode | null {
  const text = rawText.trim();
  if (text.length < 8 || text.length > MAX_INPUT_LENGTH) return null;
  if (!text.startsWith("{") || !text.endsWith("}")) return null;

  const fields = new Map<string, string>();
  for (const segment of text.slice(1, -1).split(",")) {
    const separator = segment.indexOf(":");
    if (separator <= 0) return null;
    const key = segment.slice(0, separator).trim().toLowerCase();
    // 键必须是短小写字母串；出现带引号或其它形态说明不是立创料袋格式。
    if (!/^[a-z]{1,8}$/.test(key)) return null;
    const value = segment.slice(separator + 1).trim();
    if (!fields.has(key)) fields.set(key, value);
  }

  const productCode = normalizeProductCode(fields.get("pc"));
  if (!productCode) return null;

  return {
    orderNo: normalizeText(fields.get("on")),
    productCode,
    manufacturerPart: normalizeText(fields.get("pm")),
    quantity: normalizeQuantity(fields.get("qty")),
  };
}

function normalizeProductCode(value: string | undefined): string | null {
  if (!value) return null;
  const matched = /^[Cc](\d{1,12})$/.exec(value);
  return matched ? `C${matched[1]}` : null;
}

function normalizeText(value: string | undefined): string | null {
  if (!value || value.toLowerCase() === "null") return null;
  return value;
}

function normalizeQuantity(value: string | undefined): number | null {
  if (!value || !/^\d{1,9}$/.test(value)) return null;
  const quantity = Number(value);
  return quantity > 0 ? quantity : null;
}
