import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Activity, Pause, Play, Plus, Radio, RefreshCw, ShieldAlert, Trash2 } from "lucide-react";
import { apiJson } from "../api/client";
import { useAuth } from "../auth/AuthContext";
import type { Monitor, MonitorCheck, MonitorEvent, Visibility } from "../types";

export function MonitorsPage() {
  const { user, token, hasPermission } = useAuth();
  const canWrite = hasPermission("monitors:write");
  const canDeleteAny = hasPermission("monitors:delete_any");

  const [monitors, setMonitors] = useState<Monitor[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [checks, setChecks] = useState<MonitorCheck[]>([]);
  const [sseConnected, setSseConnected] = useState(false);
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [method, setMethod] = useState("GET");
  const [expectedStatus, setExpectedStatus] = useState(200);
  const [intervalSecs, setIntervalSecs] = useState(60);
  const [visibility, setVisibility] = useState<Visibility>("internal");
  const [error, setError] = useState<string | null>(null);
  const [checkingId, setCheckingId] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const data = await apiJson<Monitor[]>("/api/v1/monitors");
      setMonitors(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể tải danh sách monitor");
    }
  }, []);

  const loadChecks = useCallback(async (id: string) => {
    try {
      const data = await apiJson<MonitorCheck[]>(`/api/v1/monitors/${id}/checks`);
      setChecks(data);
    } catch {
      setChecks([]);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (selectedId) {
      void loadChecks(selectedId);
    }
  }, [selectedId, loadChecks]);

  // Kết nối SSE real-time từ Axum
  useEffect(() => {
    if (!token) return;
    const es = new EventSource(`/api/v1/monitors/stream?token=${encodeURIComponent(token)}`);
    es.onopen = () => setSseConnected(true);
    es.onerror = () => setSseConnected(false);
    es.addEventListener("monitor_check", (ev) => {
      try {
        const event = JSON.parse((ev as MessageEvent<string>).data) as MonitorEvent;
        setMonitors((prev) =>
          prev.map((m) =>
            m.id === event.monitor_id
              ? {
                  ...m,
                  last_status: event.is_up ? "up" : "down",
                  last_status_code: event.status_code,
                  last_latency_ms: event.latency_ms,
                  last_error: event.error_message,
                  last_checked_at: event.checked_at,
                }
              : m,
          ),
        );
        if (selectedId === event.monitor_id) {
          void loadChecks(event.monitor_id);
        }
      } catch {
        // ignore malformed event
      }
    });
    return () => {
      es.close();
      setSseConnected(false);
    };
  }, [token, selectedId, loadChecks]);

  const onCreate = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await apiJson<Monitor>("/api/v1/monitors", {
        method: "POST",
        body: JSON.stringify({
          name,
          url,
          method,
          expected_status: expectedStatus,
          interval_secs: intervalSecs,
          visibility,
        }),
      });
      setName("");
      setUrl("");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể thêm monitor");
    }
  };

  const togglePause = async (m: Monitor) => {
    await apiJson(`/api/v1/monitors/${m.id}`, {
      method: "PATCH",
      body: JSON.stringify({ is_paused: !m.is_paused }),
    });
    await load();
  };

  const checkNow = async (id: string) => {
    setCheckingId(id);
    try {
      await apiJson(`/api/v1/monitors/${id}/check-now`, { method: "POST" });
      await load();
      if (selectedId === id) {
        await loadChecks(id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Ping thất bại");
    } finally {
      setCheckingId(null);
    }
  };

  const onDelete = async (id: string) => {
    await apiJson(`/api/v1/monitors/${id}`, { method: "DELETE" });
    if (selectedId === id) {
      setSelectedId(null);
      setChecks([]);
    }
    await load();
  };

  const statusBadge = (status: Monitor["last_status"], paused: boolean) => {
    if (paused) return "bg-slate-800 text-slate-400 border-slate-700";
    if (status === "up") return "bg-emerald-500/15 text-emerald-300 border-emerald-500/30";
    if (status === "down") return "bg-rose-500/15 text-rose-300 border-rose-500/30";
    return "bg-amber-500/15 text-amber-300 border-amber-500/30";
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold text-white">Giám sát Uptime Real-time</h2>
          <p className="text-sm text-slate-400">
            Worker Tokio ping song song định kỳ, tích hợp SSRF Guard chặn IP nội bộ &amp; cập nhật SSE.
          </p>
        </div>
        <span
          className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium border ${
            sseConnected
              ? "bg-emerald-500/10 text-emerald-300 border-emerald-500/30"
              : "bg-slate-800 text-slate-400 border-slate-700"
          }`}
        >
          <Radio className={`w-3.5 h-3.5 ${sseConnected ? "animate-pulse" : ""}`} />
          {sseConnected ? "SSE Real-time đang kết nối" : "Đang kết nối SSE..."}
        </span>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-300 flex items-center gap-2">
          <ShieldAlert className="w-4 h-4 shrink-0" /> {error}
        </div>
      )}

      {canWrite && (
        <form
          onSubmit={onCreate}
          className="rounded-2xl border border-slate-800 bg-slate-900/70 p-5 space-y-3"
        >
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <input
              type="text"
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Tên dịch vụ (VD: GitHub API)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white focus:border-sky-500 focus:outline-none"
            />
            <input
              type="url"
              required
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://api.github.com (SSRF Guard tự chặn IP nội bộ)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white focus:border-sky-500 focus:outline-none"
            />
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <select
              value={method}
              onChange={(e) => setMethod(e.target.value)}
              className="rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
            >
              <option value="GET">GET</option>
              <option value="HEAD">HEAD</option>
              <option value="POST">POST</option>
            </select>
            <input
              type="number"
              value={expectedStatus}
              onChange={(e) => setExpectedStatus(Number(e.target.value))}
              placeholder="Status (200)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
            />
            <input
              type="number"
              min={15}
              value={intervalSecs}
              onChange={(e) => setIntervalSecs(Number(e.target.value))}
              placeholder="Chu kỳ (giây)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
            />
            <select
              value={visibility}
              onChange={(e) => setVisibility(e.target.value as Visibility)}
              className="rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
            >
              <option value="internal">Internal</option>
              <option value="public">Public</option>
              <option value="private">Private</option>
            </select>
          </div>
          <button
            type="submit"
            className="inline-flex items-center gap-2 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-semibold px-4 py-2 text-sm cursor-pointer"
          >
            <Plus className="w-4 h-4" /> Thêm Monitor
          </button>
        </form>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {monitors.map((m) => {
          const canMutate = canDeleteAny || (canWrite && m.owner_id === user?.user_id);
          return (
            <div
              key={m.id}
              onClick={() => setSelectedId(m.id)}
              className={`rounded-2xl border p-5 transition cursor-pointer ${
                selectedId === m.id
                  ? "border-sky-500/60 bg-slate-900"
                  : "border-slate-800 bg-slate-900/70 hover:border-slate-700"
              }`}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span
                      className={`px-2 py-0.5 rounded-md text-xs font-semibold border uppercase ${statusBadge(
                        m.last_status,
                        m.is_paused,
                      )}`}
                    >
                      {m.is_paused ? "PAUSED" : m.last_status}
                    </span>
                    <h3 className="font-semibold text-white truncate">{m.name}</h3>
                  </div>
                  <p className="text-xs text-slate-400 font-mono truncate mt-1">
                    {m.method} {m.url}
                  </p>
                </div>

                {canMutate && (
                  <div
                    className="flex items-center gap-1 shrink-0"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <button
                      type="button"
                      onClick={() => void checkNow(m.id)}
                      className="p-1.5 rounded-lg hover:bg-slate-800 text-sky-400 cursor-pointer"
                      title="Ping ngay"
                    >
                      <RefreshCw
                        className={`w-4 h-4 ${checkingId === m.id ? "animate-spin" : ""}`}
                      />
                    </button>
                    <button
                      type="button"
                      onClick={() => void togglePause(m)}
                      className="p-1.5 rounded-lg hover:bg-slate-800 text-slate-300 cursor-pointer"
                      title={m.is_paused ? "Chạy tiếp" : "Tạm dừng"}
                    >
                      {m.is_paused ? <Play className="w-4 h-4" /> : <Pause className="w-4 h-4" />}
                    </button>
                    <button
                      type="button"
                      onClick={() => void onDelete(m.id)}
                      className="p-1.5 rounded-lg hover:bg-rose-500/20 text-rose-400 cursor-pointer"
                      title="Xóa"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                )}
              </div>

              <div className="mt-4 flex flex-wrap items-center gap-4 text-xs text-slate-400">
                <span>
                  HTTP:{" "}
                  <strong className="text-slate-200">{m.last_status_code ?? "—"}</strong>
                </span>
                <span>
                  Latency:{" "}
                  <strong className="text-slate-200">
                    {m.last_latency_ms !== null ? `${m.last_latency_ms} ms` : "—"}
                  </strong>
                </span>
                <span>Chu kỳ: {m.interval_secs}s</span>
              </div>
              {m.last_error && (
                <div className="mt-2 text-xs text-rose-400 truncate">Lỗi: {m.last_error}</div>
              )}
            </div>
          );
        })}
        {monitors.length === 0 && (
          <div className="lg:col-span-2 rounded-2xl border border-slate-800 bg-slate-900/50 p-10 text-center text-slate-500">
            Chưa có monitor nào. Thêm website/API ở trên để bắt đầu theo dõi real-time.
          </div>
        )}
      </div>

      {selectedId && (
        <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-5 space-y-3">
          <div className="flex items-center gap-2">
            <Activity className="w-4 h-4 text-emerald-400" />
            <h3 className="font-semibold text-white text-sm">
              Lịch sử kiểm tra gần nhất ({checks.length} lần)
            </h3>
          </div>
          <div className="flex items-end gap-1 h-14">
            {checks
              .slice(0, 40)
              .reverse()
              .map((c) => (
                <div
                  key={c.id}
                  title={`${c.is_up ? "UP" : "DOWN"} • ${c.latency_ms}ms • HTTP ${c.status_code ?? "err"}`}
                  className={`flex-1 rounded-sm ${c.is_up ? "bg-emerald-500" : "bg-rose-500"}`}
                  style={{
                    height: `${Math.max(18, Math.min(100, (c.latency_ms / 1000) * 100))}%`,
                  }}
                />
              ))}
          </div>
        </div>
      )}
    </div>
  );
}
