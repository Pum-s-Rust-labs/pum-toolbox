let accessToken: string | null = null;
let refreshHandler: (() => Promise<string | null>) | null = null;

export function setAccessToken(token: string | null): void {
  accessToken = token;
}

export function getAccessToken(): string | null {
  return accessToken;
}

export function registerRefreshHandler(fn: () => Promise<string | null>): void {
  refreshHandler = fn;
}

export class ApiClientError extends Error {
  status: number;
  code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.status = status;
    this.code = code;
  }
}

async function parseError(res: Response): Promise<ApiClientError> {
  try {
    const body = (await res.json()) as { error?: { code?: string; message?: string } };
    return new ApiClientError(
      res.status,
      body.error?.code ?? "ERROR",
      body.error?.message ?? `HTTP ${res.status}`,
    );
  } catch {
    return new ApiClientError(res.status, "HTTP_ERROR", `HTTP ${res.status}`);
  }
}

export async function apiFetch(
  path: string,
  init: RequestInit = {},
  retryOnAuth = true,
): Promise<Response> {
  const headers = new Headers(init.headers);
  if (accessToken && !headers.has("Authorization")) {
    headers.set("Authorization", `Bearer ${accessToken}`);
  }
  if (init.body && !(init.body instanceof FormData) && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  const res = await fetch(path, {
    ...init,
    headers,
    credentials: "include",
  });

  if (res.status === 401 && retryOnAuth && refreshHandler && !path.startsWith("/api/v1/auth/")) {
    const fresh = await refreshHandler();
    if (fresh) {
      headers.set("Authorization", `Bearer ${fresh}`);
      return fetch(path, { ...init, headers, credentials: "include" });
    }
  }

  return res;
}

export async function apiJson<T>(path: string, init: RequestInit = {}): Promise<T> {
  const res = await apiFetch(path, init);
  if (!res.ok) {
    throw await parseError(res);
  }
  return (await res.json()) as T;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}
