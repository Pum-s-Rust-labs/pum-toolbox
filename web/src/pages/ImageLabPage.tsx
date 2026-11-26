import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Download, Image as ImageIcon, Sparkles, Upload } from "lucide-react";
import { apiFetch, apiJson, formatBytes } from "../api/client";
import { useAuth } from "../auth/AuthContext";
import type { ImageJob } from "../types";

export function ImageLabPage() {
  const { hasPermission } = useAuth();
  const canProcess = hasPermission("images:process");

  const [files, setFiles] = useState<File[]>([]);
  const [format, setFormat] = useState<"webp" | "jpeg" | "png">("webp");
  const [quality, setQuality] = useState(82);
  const [maxWidth, setMaxWidth] = useState("");
  const [maxHeight, setMaxHeight] = useState("");
  const [grayscale, setGrayscale] = useState(false);
  const [watermark, setWatermark] = useState(false);
  const [processing, setProcessing] = useState(false);
  const [lastResult, setLastResult] = useState<{
    original: number;
    processed: number;
    durationMs: number;
    filename: string;
  } | null>(null);
  const [jobs, setJobs] = useState<ImageJob[]>([]);
  const [error, setError] = useState<string | null>(null);

  const loadJobs = useCallback(async () => {
    try {
      const data = await apiJson<ImageJob[]>("/api/v1/images/jobs");
      setJobs(data);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    void loadJobs();
  }, [loadJobs]);

  const onProcess = async (e: FormEvent) => {
    e.preventDefault();
    if (files.length === 0) return;
    setError(null);
    setProcessing(true);

    try {
      const fd = new FormData();
      for (const file of files) {
        fd.append("files", file);
      }
      fd.append("format", format);
      fd.append("quality", String(quality));
      if (maxWidth) fd.append("max_width", maxWidth);
      if (maxHeight) fd.append("max_height", maxHeight);
      fd.append("grayscale", String(grayscale));
      fd.append("watermark", String(watermark));

      const res = await apiFetch("/api/v1/images/process", {
        method: "POST",
        body: fd,
      });

      if (!res.ok) {
        const body = (await res.json().catch(() => ({}))) as { error?: { message?: string } };
        throw new Error(body.error?.message ?? `HTTP ${res.status}`);
      }

      const blob = await res.blob();
      const disp = res.headers.get("Content-Disposition") ?? "";
      const match = /filename="([^"]+)"/.exec(disp);
      const filename = match?.[1] ?? `output.${files.length > 1 ? "zip" : format}`;

      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);

      setLastResult({
        original: Number(res.headers.get("X-Pum-Original-Bytes") ?? 0),
        processed: Number(res.headers.get("X-Pum-Processed-Bytes") ?? blob.size),
        durationMs: Number(res.headers.get("X-Pum-Duration-Ms") ?? 0),
        filename,
      });
      await loadJobs();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Xử lý ảnh thất bại");
    } finally {
      setProcessing(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white">ImageLab — Xử lý &amp; Nén ảnh song song</h2>
        <p className="text-sm text-slate-400">
          Chạy đa nhân CPU bằng Rust `rayon` + `image`, tự kiểm tra magic bytes &amp; chống decompression bomb.
        </p>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-300">
          {error}
        </div>
      )}

      {!canProcess ? (
        <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 p-5 text-sm text-amber-300">
          Vai trò hiện tại của bạn chưa có quyền <code className="font-mono">images:process</code>.
        </div>
      ) : (
        <form
          onSubmit={onProcess}
          className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6 space-y-5"
        >
          <label className="flex flex-col items-center justify-center rounded-2xl border-2 border-dashed border-slate-700 hover:border-amber-400/60 bg-slate-950/60 p-8 cursor-pointer transition">
            <Upload className="w-8 h-8 text-amber-400 mb-2" />
            <span className="text-sm font-medium text-white">
              Chọn 1 hoặc nhiều ảnh (PNG, JPG, WebP, GIF — tối đa 10 ảnh/lần)
            </span>
            <span className="text-xs text-slate-400 mt-1">
              {files.length > 0
                ? `Đã chọn ${files.length} file (${formatBytes(files.reduce((s, f) => s + f.size, 0))})`
                : "Nhiều ảnh sẽ tự động đóng gói thành file .zip"}
            </span>
            <input
              type="file"
              accept="image/*"
              multiple
              className="hidden"
              onChange={(e) => setFiles(Array.from(e.target.files ?? []))}
            />
          </label>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <div>
              <label className="block text-xs text-slate-400 mb-1">Định dạng đầu ra</label>
              <select
                value={format}
                onChange={(e) => setFormat(e.target.value as "webp" | "jpeg" | "png")}
                className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
              >
                <option value="webp">WebP (Nhẹ nhất)</option>
                <option value="jpeg">JPEG</option>
                <option value="png">PNG (Không mất nét)</option>
              </select>
            </div>
            <div>
              <label className="block text-xs text-slate-400 mb-1">
                Chất lượng nén ({quality}%)
              </label>
              <input
                type="range"
                min={10}
                max={100}
                value={quality}
                onChange={(e) => setQuality(Number(e.target.value))}
                className="w-full accent-amber-400 mt-2"
              />
            </div>
            <div>
              <label className="block text-xs text-slate-400 mb-1">Max Width (px)</label>
              <input
                type="number"
                min={16}
                value={maxWidth}
                onChange={(e) => setMaxWidth(e.target.value)}
                placeholder="Giữ nguyên"
                className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
              />
            </div>
            <div>
              <label className="block text-xs text-slate-400 mb-1">Max Height (px)</label>
              <input
                type="number"
                min={16}
                value={maxHeight}
                onChange={(e) => setMaxHeight(e.target.value)}
                placeholder="Giữ nguyên"
                className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"
              />
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-6 text-sm text-slate-300">
            <label className="inline-flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={grayscale}
                onChange={(e) => setGrayscale(e.target.checked)}
                className="accent-amber-400"
              />
              Chuyển đen trắng (Grayscale)
            </label>
            <label className="inline-flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={watermark}
                onChange={(e) => setWatermark(e.target.checked)}
                className="accent-amber-400"
              />
              Đóng dấu dải watermark góc ảnh
            </label>
          </div>

          <button
            type="submit"
            disabled={processing || files.length === 0}
            className="inline-flex items-center gap-2 rounded-xl bg-amber-400 hover:bg-amber-300 disabled:opacity-50 text-slate-950 font-semibold px-5 py-2.5 text-sm cursor-pointer"
          >
            <Sparkles className="w-4 h-4" />
            {processing ? "Đang xử lý song song bằng Rayon..." : "Xử lý & Tải xuống"}
          </button>
        </form>
      )}

      {lastResult && (
        <div className="rounded-2xl border border-emerald-500/30 bg-emerald-500/10 p-4 flex flex-wrap items-center justify-between gap-3 text-sm">
          <div className="flex items-center gap-2 text-emerald-300">
            <Download className="w-4 h-4" />
            Đã tải xuống <strong>{lastResult.filename}</strong>
          </div>
          <div className="text-xs text-slate-200 flex gap-4">
            <span>Gốc: {formatBytes(lastResult.original)}</span>
            <span>Sau nén: {formatBytes(lastResult.processed)}</span>
            <span>Thời gian Rust: {lastResult.durationMs} ms</span>
          </div>
        </div>
      )}

      <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-5">
        <div className="flex items-center gap-2 mb-3">
          <ImageIcon className="w-4 h-4 text-amber-400" />
          <h3 className="font-semibold text-white text-sm">Lịch sử xử lý gần đây</h3>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-xs">
            <thead className="text-slate-400 uppercase border-b border-slate-800">
              <tr>
                <th className="py-2 text-left">Số file</th>
                <th className="py-2 text-left">Định dạng</th>
                <th className="py-2 text-right">Gốc</th>
                <th className="py-2 text-right">Sau nén</th>
                <th className="py-2 text-right">Thời gian</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/70">
              {jobs.map((j) => (
                <tr key={j.id}>
                  <td className="py-2 text-white">{j.file_count} file</td>
                  <td className="py-2 uppercase font-mono text-amber-300">
                    {j.output_format} (Q{j.quality})
                  </td>
                  <td className="py-2 text-right text-slate-400">
                    {formatBytes(j.original_bytes)}
                  </td>
                  <td className="py-2 text-right text-emerald-400 font-medium">
                    {formatBytes(j.processed_bytes)}
                  </td>
                  <td className="py-2 text-right font-mono text-slate-300">{j.duration_ms} ms</td>
                </tr>
              ))}
              {jobs.length === 0 && (
                <tr>
                  <td colSpan={5} className="py-6 text-center text-slate-500">
                    Chưa có lịch sử xử lý ảnh.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
