// 本文件拥有 Shell Bridge 传输错误的统一规范化；它不执行平台调用，也不解释业务错误文案。

/** 可被前端恢复流程消费的 Shell Bridge 错误。 */
export class ShellBridgeTransportError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "ShellBridgeTransportError";
    this.code = code;
  }
}

/**
 * 把 Tauri invoke 的字符串、对象或 Error rejection 统一为带稳定 code 的错误。
 * Rust command 当前以 JSON 字符串返回错误；保留对象形状兼容未来 Tauri 序列化调整。
 */
export function normalizeShellBridgeTransportError(error: unknown): ShellBridgeTransportError {
  const candidate = extractErrorCandidate(error);
  if (candidate) {
    return new ShellBridgeTransportError(candidate.code, candidate.message);
  }

  const message = error instanceof Error && error.message ? error.message : "Shell Bridge 调用失败";
  return new ShellBridgeTransportError("invalid_bridge_payload", message);
}

function extractErrorCandidate(error: unknown): { code: string; message: string } | null {
  if (isRecord(error)) {
    return readErrorCandidate(error);
  }

  if (error instanceof Error) {
    const errorCode = readErrorCode(error);
    if (errorCode) {
      return { code: errorCode, message: error.message || "Shell Bridge 调用失败" };
    }
    return parseErrorText(error.message);
  }

  if (typeof error === "string") {
    return parseErrorText(error);
  }

  return null;
}

function parseErrorText(value: string): { code: string; message: string } | null {
  try {
    const parsed: unknown = JSON.parse(value);
    return isRecord(parsed) ? readErrorCandidate(parsed) : null;
  } catch {
    return null;
  }
}

function readErrorCandidate(value: Record<string, unknown>): { code: string; message: string } | null {
  return typeof value.code === "string" && typeof value.message === "string"
    ? { code: value.code, message: value.message }
    : null;
}

function readErrorCode(value: Error): string | null {
  const code = (value as Error & { code?: unknown }).code;
  return typeof code === "string" && code ? code : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
