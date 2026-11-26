import { useCallback, useEffect, useState, type FormEvent } from "react";
import { BarChart3, Copy, ExternalLink, Lock, Plus, QrCode, Trash2, X } from "lucide-react";
import { apiFetch, apiJson } from "../api/client";
import { useAuth } from "../auth/AuthContext";
import type { LinkAnalytics, ShortLink, Visibility } from "../types";

export function LinksPage() {
  const { user, hasPermission } = useAuth();
  const canWrite = hasPermission("links:write");
  const canDeleteAny = hasPermission("links:delete_any");

  const [links, setLinks] = useState<ShortLink[]>([]);
  const [targetUrl, setTargetUrl] = useState("");
  const [slug, setSlug] = useState("");
  const [title, setTitle] = useState("");
  const [visibility, setVisibility] = useState<Visibility>("public");
  const [password, setPassword] = useState("");
  const [expiresHours, setExpiresHours] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [qrModal, setQrModal] = useState<{ slug: string; svg: string } | null>(null);
  const [analytics, setAnalytics] = useState<LinkAnalytics | null>(null);

  const load = useCallback(async () => {
    try {
      const data = await apiJson<ShortLink[]>("/api/v1/links");
      setLinks(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể tải danh sách link");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const onCreate = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await apiJson<ShortLink>("/api/v1/links", {
        method: "POST",
        body: JSON.stringify({
          target_url: targetUrl,
          slug: slug || undefined,
          title: title || undefined,
          visibility,
          password: password || undefined,
          expires_in_hours: expiresHours ? Number(expiresHours) : undefined,
        }),
      });
      setTargetUrl("");
      setSlug("");
      setTitle("");
      setPassword("");
      setExpiresHours("");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Tạo shortlink thất bại");
    }
  };

  const onDelete = async (id: string) => {
    setError(null);
    try {
      await apiJson(`/api/v1/links/${id}`, { method: "DELETE" });
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Xóa thất bại");
    }
  };

  const openQr = async (link: ShortLink) => {
    const res = await apiFetch(`/api/v1/links/${link.id}/qr`);
    if (res.ok) {
      const svg = await res.text();
      setQrModal({ slug: link.slug, svg });
    }
  };

  const openAnalytics = async (id: string) => {
    try {
      const data = await apiJson<LinkAnalytics>(`/api/v1/links/${id}/analytics`);
      setAnalytics(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể tải thống kê");
    }
  };

  const copyShortUrl = (link: ShortLink) => {
    const full = `${window.location.origin}/r/${link.slug}`;
    void navigator.clipboard.writeText(full);
    setCopiedId(link.id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white">Rút gọn Link &amp; Thống kê</h2>
        <p className="text-sm text-slate-400">
          Tạo liên kết rút gọn siêu tốc, mã QR SVG, mật khẩu bảo vệ và theo dõi lượt click.
        </p>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-300">
          {error}
        </div>
      )}

      {canWrite && (
        <form
          onSubmit={onCreate}
          className="rounded-2xl border border-slate-800 bg-slate-900/70 p-5 space-y-4"
        >
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <input
              type="url"
              required
              value={targetUrl}
              onChange={(e) => setTargetUrl(e.target.value)}
              placeholder="URL đích (https://...)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white focus:border-sky-500 focus:outline-none"
            />
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Tiêu đề mô tả (tuỳ chọn)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white focus:border-sky-500 focus:outline-none"
            />
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
            <input
              type="text"
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="Custom slug (tuỳ chọn)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2 text-sm text-white focus:border-sky-500 focus:outline-none"
            />
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Mật khẩu bảo vệ (tuỳ chọn)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2 text-sm text-white focus:border-sky-500 focus:outline-none"
            />
            <input
              type="number"
              min={1}
              value={expiresHours}
              onChange={(e) => setExpiresHours(e.target.value)}
              placeholder="Hết hạn sau (giờ)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2 text-sm text-white focus:border-sky-500 focus:outline-none"
            />
            <select
              value={visibility}
              onChange={(e) => setVisibility(e.target.value as Visibility)}
              className="rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2 text-sm text-white focus:border-sky-500 focus:outline-none"
            >
              <option value="public">Public</option>
              <option value="internal">Internal</option>
              <option value="private">Private</option>
            </select>
          </div>
          <button
            type="submit"
            className="inline-flex items-center gap-2 rounded-xl bg-sky-500 hover:bg-sky-400 text-slate-950 font-semibold px-4 py-2 text-sm cursor-pointer"
          >
            <Plus className="w-4 h-4" /> Tạo Shortlink
          </button>
        </form>
      )}

      <div className="rounded-2xl border border-slate-800 bg-slate-900/70 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-slate-950/70 text-slate-400 text-xs uppercase">
              <tr>
                <th className="px-4 py-3 text-left">Shortlink</th>
                <th className="px-4 py-3 text-left">Đích đến</th>
                <th className="px-4 py-3 text-center">Clicks</th>
                <th className="px-4 py-3 text-center">Hiển thị</th>
                <th className="px-4 py-3 text-right">Thao tác</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800">
              {links.map((link) => {
                const canDelete = canDeleteAny || (canWrite && link.owner_id === user?.user_id);
                return (
                  <tr key={link.id} className="hover:bg-slate-800/30">
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-sky-400 font-medium">/r/{link.slug}</span>
                        {link.has_password && <Lock className="w-3.5 h-3.5 text-amber-400" />}
                      </div>
                      {link.title && <div className="text-xs text-slate-400">{link.title}</div>}
                    </td>
                    <td className="px-4 py-3 max-w-xs truncate text-slate-300">
                      <a
                        href={link.target_url}
                        target="_blank"
                        rel="noreferrer"
                        className="inline-flex items-center gap-1 hover:text-sky-300"
                      >
                        {link.target_url} <ExternalLink className="w-3 h-3 shrink-0" />
                      </a>
                    </td>
                    <td className="px-4 py-3 text-center font-semibold text-white">
                      {link.click_count}
                    </td>
                    <td className="px-4 py-3 text-center">
                      <span className="px-2 py-0.5 rounded-md bg-slate-800 text-xs text-slate-300">
                        {link.visibility}
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex items-center justify-end gap-1.5">
                        <button
                          type="button"
                          onClick={() => copyShortUrl(link)}
                          className="p-1.5 rounded-lg hover:bg-slate-800 text-slate-300 cursor-pointer"
                          title="Copy link"
                        >
                          {copiedId === link.id ? (
                            <span className="text-xs text-emerald-400 font-medium">Copied</span>
                          ) : (
                            <Copy className="w-4 h-4" />
                          )}
                        </button>
                        <button
                          type="button"
                          onClick={() => void openQr(link)}
                          className="p-1.5 rounded-lg hover:bg-slate-800 text-slate-300 cursor-pointer"
                          title="Mã QR"
                        >
                          <QrCode className="w-4 h-4" />
                        </button>
                        <button
                          type="button"
                          onClick={() => void openAnalytics(link.id)}
                          className="p-1.5 rounded-lg hover:bg-slate-800 text-sky-400 cursor-pointer"
                          title="Thống kê"
                        >
                          <BarChart3 className="w-4 h-4" />
                        </button>
                        {canDelete && (
                          <button
                            type="button"
                            onClick={() => void onDelete(link.id)}
                            className="p-1.5 rounded-lg hover:bg-rose-500/20 text-rose-400 cursor-pointer"
                            title="Xóa"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
              {links.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-4 py-8 text-center text-slate-500">
                    Chưa có shortlink nào. Hãy tạo liên kết đầu tiên ở trên!
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {qrModal && (
        <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4">
          <div className="rounded-2xl border border-slate-700 bg-slate-900 p-6 max-w-sm w-full text-center space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="font-semibold text-white">Mã QR — /r/{qrModal.slug}</h3>
              <button
                type="button"
                onClick={() => setQrModal(null)}
                className="text-slate-400 hover:text-white cursor-pointer"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            <div
              className="mx-auto w-60 h-60 rounded-xl bg-white p-2 flex items-center justify-center"
              dangerouslySetInnerHTML={{ __html: qrModal.svg }}
            />
          </div>
        </div>
      )}

      {analytics && (
        <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4">
          <div className="rounded-2xl border border-slate-700 bg-slate-900 p-6 max-w-lg w-full space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-semibold text-white">Thống kê /r/{analytics.link.slug}</h3>
                <p className="text-xs text-slate-400">{analytics.short_url}</p>
              </div>
              <button
                type="button"
                onClick={() => setAnalytics(null)}
                className="text-slate-400 hover:text-white cursor-pointer"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="rounded-xl bg-slate-950 p-3 border border-slate-800">
                <div className="text-xs text-slate-400">Tổng lượt click</div>
                <div className="text-2xl font-bold text-sky-400">{analytics.link.click_count}</div>
              </div>
              <div className="rounded-xl bg-slate-950 p-3 border border-slate-800">
                <div className="text-xs text-slate-400">Thiết bị truy cập</div>
                <div className="mt-1 space-y-1 text-xs">
                  {analytics.devices.length === 0 ? (
                    <span className="text-slate-500">Chưa có dữ liệu</span>
                  ) : (
                    analytics.devices.map((d) => (
                      <div key={d.device_type} className="flex justify-between">
                        <span className="capitalize text-slate-300">{d.device_type}</span>
                        <span className="font-semibold text-white">{d.clicks}</span>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
            <div className="rounded-xl bg-slate-950 p-3 border border-slate-800">
              <div className="text-xs text-slate-400 mb-2">Lượt click 14 ngày gần nhất</div>
              {analytics.daily.length === 0 ? (
                <div className="text-xs text-slate-500">Chưa có lượt truy cập nào.</div>
              ) : (
                <div className="space-y-1.5">
                  {analytics.daily.map((d) => (
                    <div key={d.day} className="flex items-center gap-2 text-xs">
                      <span className="w-24 font-mono text-slate-400">{d.day}</span>
                      <div className="flex-1 h-2 bg-slate-800 rounded-full overflow-hidden">
                        <div
                          className="h-full bg-sky-500 rounded-full"
                          style={{
                            width: `${Math.min(100, (d.clicks / Math.max(1, analytics.link.click_count)) * 100)}%`,
                          }}
                        />
                      </div>
                      <span className="w-8 text-right font-semibold text-white">{d.clicks}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
