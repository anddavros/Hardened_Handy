export interface ParsedTauriError {
  code: string;
  message: string;
  detail?: string;
  raw: unknown;
}

const DEFAULT_ERROR_CODE = "unknown_error";

export function parseTauriError(
  error: unknown,
  fallbackMessage = "An unexpected error occurred.",
): ParsedTauriError {
  if (typeof error === "string") {
    return {
      code: DEFAULT_ERROR_CODE,
      message: error || fallbackMessage,
      raw: error,
    };
  }

  if (error && typeof error === "object") {
    const errObject = error as Record<string, unknown>;
    const payload = (errObject.payload ?? errObject) as Record<string, unknown>;

    const code = typeof payload.code === "string" ? payload.code : DEFAULT_ERROR_CODE;
    const messageCandidate =
      (typeof payload.message === "string" && payload.message) ||
      (typeof errObject.message === "string" && errObject.message) ||
      undefined;
    const detail =
      (typeof payload.detail === "string" && payload.detail) ||
      (typeof payload.error === "string" && payload.error) ||
      (typeof payload.details === "string" && payload.details) ||
      undefined;

    return {
      code,
      message: messageCandidate || fallbackMessage,
      detail,
      raw: error,
    };
  }

  return {
    code: DEFAULT_ERROR_CODE,
    message: fallbackMessage,
    raw: error,
  };
}
