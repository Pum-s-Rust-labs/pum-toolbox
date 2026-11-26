import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { apiFetch, apiJson, registerRefreshHandler, setAccessToken } from "../api/client";
import type { AuthResponse, Permission, Principal } from "../types";

interface AuthContextValue {
  user: Principal | null;
  token: string | null;
  loading: boolean;
  login: (identifier: string, password: string) => Promise<void>;
  register: (email: string, username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  hasPermission: (perm: Permission) => boolean;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<Principal | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const applyAuth = useCallback((data: AuthResponse) => {
    setAccessToken(data.access_token);
    setToken(data.access_token);
    setUser(data.user);
  }, []);

  const clearAuth = useCallback(() => {
    setAccessToken(null);
    setToken(null);
    setUser(null);
  }, []);

  const refreshSession = useCallback(async (): Promise<string | null> => {
    try {
      const res = await apiFetch("/api/v1/auth/refresh", { method: "POST" }, false);
      if (!res.ok) {
        clearAuth();
        return null;
      }
      const data = (await res.json()) as AuthResponse;
      applyAuth(data);
      return data.access_token;
    } catch {
      clearAuth();
      return null;
    }
  }, [applyAuth, clearAuth]);

  useEffect(() => {
    registerRefreshHandler(refreshSession);
    void refreshSession().finally(() => setLoading(false));
  }, [refreshSession]);

  const login = useCallback(
    async (identifier: string, password: string) => {
      const data = await apiJson<AuthResponse>("/api/v1/auth/login", {
        method: "POST",
        body: JSON.stringify({ identifier, password }),
      });
      applyAuth(data);
    },
    [applyAuth],
  );

  const register = useCallback(
    async (email: string, username: string, password: string) => {
      const data = await apiJson<AuthResponse>("/api/v1/auth/register", {
        method: "POST",
        body: JSON.stringify({ email, username, password }),
      });
      applyAuth(data);
    },
    [applyAuth],
  );

  const logout = useCallback(async () => {
    try {
      await apiFetch("/api/v1/auth/logout", { method: "POST" }, false);
    } finally {
      clearAuth();
    }
  }, [clearAuth]);

  const hasPermission = useCallback(
    (perm: Permission) => Boolean(user?.permissions.includes(perm)),
    [user],
  );

  return (
    <AuthContext.Provider value={{ user, token, loading, login, register, logout, hasPermission }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth must be used inside AuthProvider");
  }
  return ctx;
}
