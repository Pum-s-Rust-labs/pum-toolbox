import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Copy, Eye, Flame, Plus, Search, Trash2, X } from "lucide-react";
import { apiJson } from "../api/client";
import { useAuth } from "../auth/AuthContext";
import type { PasteDetail, PasteSummary, Visibility } from "../types";

export function PastesPage() {
  const { user, hasPermission } = useAuth();
  const canWrite = hasPermission("pastes:write");
  const canDeleteAny = hasPermission("pastes:delete_any");

  const [pastes, setPastes] = useState<PasteSummary[]>([]);
  const [query, setQuery] = useState("");
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [language, setLanguage] = useState("markdown");
  const [tags, setTags] = useState("");
  const [visibility, setVisibility] = useState<Visibility>("internal");
  const [maxViews, setMaxViews] = useState("");
  const [expiresMinutes, setExpiresMinutes] = useState("");
  const [activePaste, setActivePaste] = useState<PasteDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (search = "") => {
    try {
      const qs = search.trim() ? `?q=${encodeURIComponent(search.trim())}` : "";
      const data = await apiJson<PasteSummary[]>(`/api/v1/pastes${qs}`);
      setPastes(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể tải danh sách paste");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const onCreate = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await apiJson<PasteDetail>("/api/v1/pastes", {
        method: "POST",
        body: JSON.stringify({
          title,
          content,
          language,
          tags: tags
            .split(",")
            .map((t) => t.trim())
            .filter(Boolean),
          visibility,
          max_views: maxViews ? Number(maxViews) : undefined,
          expires_in_minutes: expiresMinutes ? Number(expiresMinutes) : undefined,
        }),
      });
      setTitle("");
      setContent("");
      setTags("");
      setMaxViews("");
      setExpiresMinutes("");
      await load(query);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Tạo paste thất bại");
    }
  };

  const openPaste = async (slug: string) => {
    setError(null);
    try {
      const detail = await apiJson<PasteDetail>(`/api/v1/pastes/${slug}`);
      setActivePaste(detail);
      await load(query);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể mở paste");
    }
  };

  const onDelete = async (id: string) => {
    await apiJson(`/api/v1/pastes/id/${id}`, { method: "DELETE" });
    if (activePaste?.id === id) setActivePaste(null);
    await load(query);
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold text-white">Code &amp; Markdown Pastebin</h2>
          <p className="text-sm text-slate-400">
            Render Markdown đã lọc XSS bằng `ammonia`, hỗ trợ tự hủy sau N lượt xem (burn-after-reading).
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search className="w-4 h-4 text-slate-500 absolute left-3 top-2.5" />
            <input
              type="text"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                void load(e.target.value);
              }}
              placeholder="Tìm tiêu đề, nội dung, tag..."
              className="rounded-xl border border-slate-700 bg-slate-900 pl-9 pr-3 py-2 text-sm text-white focus:border-violet-500 focus:outline-none"
            />
          </div>
        </div>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-300">
          {error}
        </div>
      )}

      {canWrite && (
        <form
          onSubmit={onCreate}
          className="rounded-2xl border border-slate-800 bg-slate-900/70 p-5 space-y-3"
        >
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <input
              type="text"
              required
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Tiêu đề snippet / ghi chú"
              className="md:col-span-2 rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white focus:border-violet-500 focus:outline-none"
            />
            <select
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              className="rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
            >
              <option value="markdown">Markdown</option>
              <option value="rust">Rust</option>
              <option value="typescript">TypeScript</option>
              <option value="sql">SQL</option>
              <option value="json">JSON</option>
              <option value="plaintext">Plaintext</option>
            </select>
          </div>
          <textarea
            required
            rows={5}
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder="# Ghi chú Markdown hoặc dán mã nguồn vào đây..."
            className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm font-mono text-white focus:border-violet-500 focus:outline-none"
          />
          <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
            <input
              type="text"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="Tags (rust, notes, secret)"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
            />
            <input
              type="number"
              min={1}
              value={maxViews}
              onChange={(e) => setMaxViews(e.target.value)}
              placeholder="Tự hủy sau N lượt xem"
              className="rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
            />
            <input
              type="number"
              min={1}
              value={expiresMinutes}
              onChange={(e) => setExpiresMinutes(e.target.value)}
              placeholder="Hết hạn sau (phút)"
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
            className="inline-flex items-center gap-2 rounded-xl bg-violet-500 hover:bg-violet-400 text-white font-semibold px-4 py-2 text-sm cursor-pointer"
          >
            <Plus className="w-4 h-4" /> Lưu Paste
          </button>
        </form>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {pastes.map((p) => {
          const canDelete = canDeleteAny || (canWrite && p.owner_id === user?.user_id);
          return (
            <div
              key={p.id}
              className="rounded-2xl border border-slate-800 bg-slate-900/70 p-4 flex flex-col justify-between gap-3"
            >
              <div>
                <div className="flex items-start justify-between gap-2">
                  <h3 className="font-semibold text-white">{p.title}</h3>
                  <span className="px-2 py-0.5 rounded-md bg-slate-800 text-xs font-mono text-violet-300">
                    {p.language}
                  </span>
                </div>
                <div className="flex flex-wrap gap-1.5 mt-2">
                  {p.tags.map((t) => (
                    <span
                      key={t}
                      className="px-2 py-0.5 rounded-md bg-slate-950 border border-slate-800 text-xs text-slate-400"
                    >
                      #{t}
                    </span>
                  ))}
                  {p.max_views && (
                    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-amber-500/10 text-amber-300 text-xs">
                      <Flame className="w-3 h-3" /> {p.view_count}/{p.max_views} lượt
                    </span>
                  )}
                </div>
              </div>
              <div className="flex items-center justify-between pt-2 border-t border-slate-800 text-xs text-slate-400">
                <span>
                  {p.visibility} · {p.view_count} lượt xem
                </span>
                <div className="flex items-center gap-1.5">
                  <button
                    type="button"
                    onClick={() => void openPaste(p.slug)}
                    className="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-white cursor-pointer"
                  >
                    <Eye className="w-3.5 h-3.5" /> Xem
                  </button>
                  {canDelete && (
                    <button
                      type="button"
                      onClick={() => void onDelete(p.id)}
                      className="p-1.5 rounded-lg hover:bg-rose-500/20 text-rose-400 cursor-pointer"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {activePaste && (
        <div className="fixed inset-0 z-50 bg-black/75 flex items-center justify-center p-4">
          <div className="rounded-2xl border border-slate-700 bg-slate-900 max-w-3xl w-full max-h-[85vh] flex flex-col overflow-hidden">
            <div className="flex items-center justify-between px-5 py-4 border-b border-slate-800">
              <div>
                <h3 className="font-bold text-white">{activePaste.title}</h3>
                <p className="text-xs text-slate-400">
                  {activePaste.language} · {activePaste.view_count}
                  {activePaste.max_views ? `/${activePaste.max_views}` : ""} lượt xem
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => void navigator.clipboard.writeText(activePaste.content)}
                  className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-xs text-white cursor-pointer"
                >
                  <Copy className="w-3.5 h-3.5" /> Copy
                </button>
                <button
                  type="button"
                  onClick={() => setActivePaste(null)}
                  className="text-slate-400 hover:text-white cursor-pointer"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>
            </div>
            <div className="p-5 overflow-y-auto">
              {activePaste.rendered_html ? (
                <div
                  className="markdown-body"
                  dangerouslySetInnerHTML={{ __html: activePaste.rendered_html }}
                />
              ) : (
                <pre className="rounded-xl bg-slate-950 border border-slate-800 p-4 text-sm font-mono text-slate-200 overflow-x-auto whitespace-pre-wrap">
                  {activePaste.content}
                </pre>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
