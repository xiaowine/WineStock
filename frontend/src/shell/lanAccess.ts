// 本文件只筛选 Shell 快照中的真实局域网访问地址；它不枚举网卡、不推导监听地址，也不管理 UI 状态。
import type { RuntimeSnapshot } from "./contract";

const PLACEHOLDER_MARKERS = /[<>{}]/u;

/** 地址入口只依赖快照中的最小只读结构，兼容 Vue readonly 快照。 */
interface LanAccessSnapshot {
  readonly config: {
    readonly mode: RuntimeSnapshot["config"]["mode"];
  };
  readonly service: {
    readonly ownership: RuntimeSnapshot["service"]["ownership"];
    readonly phase: RuntimeSnapshot["service"]["phase"];
    readonly lanAccessUrls?: readonly string[];
  };
  readonly capabilities: {
    readonly serverMode: boolean;
  };
}

/**
 * 返回当前快照中可以向其它设备展示的局域网访问 URL。
 * 地址顺序由 Shell 决定；前端只规范化、过滤和去重。
 */
export function getUsableLanAccessUrls(snapshot: LanAccessSnapshot | null | undefined): string[] {
  if (
    !snapshot ||
    snapshot.config.mode !== "server-mode" ||
    snapshot.service.ownership !== "local" ||
    snapshot.service.phase !== "running" ||
    !snapshot.capabilities.serverMode
  ) {
    return [];
  }

  return normalizeLanAccessUrls(snapshot.service.lanAccessUrls);
}

/** 对 Shell 提供的地址执行防御性清理，不把 wildcard、loopback 或占位值交给界面。 */
export function normalizeLanAccessUrls(urls: readonly string[] | undefined): string[] {
  if (!urls?.length) return [];

  const normalized: string[] = [];
  const seen = new Set<string>();
  for (const rawUrl of urls) {
    const candidate = rawUrl.trim();
    if (!candidate || PLACEHOLDER_MARKERS.test(candidate) || candidate.includes("局域网地址")) {
      continue;
    }

    let parsed: URL;
    try {
      parsed = new URL(candidate);
    } catch {
      continue;
    }

    if (
      (parsed.protocol !== "http:" && parsed.protocol !== "https:") ||
      parsed.username ||
      parsed.password ||
      parsed.pathname !== "/" ||
      parsed.search ||
      parsed.hash
    ) {
      continue;
    }

    const hostname = normalizeHostname(parsed.hostname);
    if (!hostname || isWildcardHost(hostname) || isLoopbackHost(hostname)) continue;

    const origin = parsed.origin;
    if (origin === "null" || seen.has(origin)) continue;
    seen.add(origin);
    normalized.push(origin);
  }

  return normalized;
}

function normalizeHostname(hostname: string): string {
  let normalized = hostname.trim().toLowerCase();
  if (normalized.startsWith("[") && normalized.endsWith("]")) {
    normalized = normalized.slice(1, -1);
  }
  return normalized.endsWith(".") ? normalized.slice(0, -1) : normalized;
}

function isWildcardHost(hostname: string): boolean {
  return hostname === "0.0.0.0" || hostname === "::" || hostname === "0:0:0:0:0:0:0:0";
}

function isLoopbackHost(hostname: string): boolean {
  if (
    hostname === "localhost" ||
    hostname.endsWith(".localhost") ||
    hostname === "::1" ||
    hostname === "0:0:0:0:0:0:0:1"
  ) {
    return true;
  }

  const ipv4Segments = hostname.split(".");
  return ipv4Segments.length === 4 && ipv4Segments[0] === "127";
}
