import { useEffect, useState } from "react";
import { Activity, FileCode2, Image as ImageIcon, Link2, Shield, Users } from "lucide-react";
import { apiJson, formatBytes } from "../api/client";
import { useAuth } from "../auth/AuthContext";
import type { DashboardSummary } from "../types";

export function DashboardPage({ onNavigate }: { onNavigate: (tab: string) => void }) {
  const { user } = useAuth();
  const [summary, setSummary] = useState<DashboardSummary | null>(null);

  useEffect(() => {
    void apiJson<DashboardSummary>("/api/v1/dashboard/summary")
      .then(setSummary)
      .catch(() => {});
  }, []);

  const cards = [
    {
      title: "Shortlinks & Clicks",
      value: summary ? `${summary.total_links} link` : "—",
      sub: summary ? `${summary.total_clicks} tổng lượt click` : "",
      icon: Link2,
      color: "text-sky-400 bg-sky-500/10 border-sky-500/20",
      tab: "links",
    },
    {
      title: "Uptime Monitors",
      value: summary ? `${summary.monitors_up}/${summary.total_monitors} UP` : "—",
      sub: summary ? `${summary.monitors_down} đang sự cố` : "",
      icon: Activity,
      color: "text-emerald-400 bg-emerald-500/10 border-emerald-500/20",
      tab: "monitors",
    },
    {
      title: "Code & Markdown Pastes",
      value: summary ? `${summary.total_pastes} paste` : "—",
      sub: "Hỗ trợ tự hủy & lọc XSS",
      icon: FileCode2,
      color: "text-violet-400 bg-violet-500/10 border-violet-500/20",
      tab: "pastes",
    },
    {
      title: "ImageLab (Rayon)",
      value: summary ? `${summary.total_image_jobs} lượt xử lý` : "—",
      sub: summary ? `Tiết kiệm ${formatBytes(summary.total_bytes_saved)}` : "",
      icon: ImageIcon,
      color: "text-amber-400 bg-amber-500/10 border-amber-500/20",
      tab: "images",
    },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white">Tổng quan hệ thống</h2>
        <p className="text-sm text-slate-400 mt-1">
          Chào mừng <span className="text-sky-400 font-medium">@{user?.username}</span> — quản lý
          tập trung cả 4 công cụ Dev &amp; Ops chạy trên nền Rust + PostgreSQL.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
        {cards.map((c) => {
          const Icon = c.icon;
          return (
            <button
              key={c.tab}
              type="button"
              onClick={() => onNavigate(c.tab)}
              className="text-left rounded-2xl border border-slate-800 bg-slate-900/70 hover:border-slate-700 p-5 transition cursor-pointer group"
            >
              <div className="flex items-center justify-between mb-3">
                <span className="text-xs font-medium text-slate-400">{c.title}</span>
                <span className={`p-2 rounded-xl border ${c.color}`}>
                  <Icon className="w-4 h-4" />
                </span>
              </div>
              <div className="text-2xl font-bold text-white group-hover:text-sky-300 transition">
                {c.value}
              </div>
              <div className="text-xs text-slate-500 mt-1">{c.sub}</div>
            </button>
          );
        })}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2 rounded-2xl border border-slate-800 bg-slate-900/70 p-5">
          <div className="flex items-center gap-2 mb-3">
            <Shield className="w-5 h-5 text-sky-400" />
            <h3 className="font-semibold text-white">
              Quyền hiệu lực của bạn ({user?.role.toUpperCase()})
            </h3>
          </div>
          <div className="flex flex-wrap gap-2">
            {user?.permissions.map((perm) => (
              <span
                key={perm}
                className="px-2.5 py-1 rounded-lg bg-slate-950 border border-slate-800 text-xs font-mono text-sky-300"
              >
                {perm}
              </span>
            ))}
          </div>
        </div>

        <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-5 flex flex-col justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <Users className="w-5 h-5 text-emerald-400" />
              <h3 className="font-semibold text-white">Cộng đồng &amp; Bảo mật</h3>
            </div>
            <p className="text-sm text-slate-400">
              Tổng số tài khoản trong tổ chức:{" "}
              <strong className="text-white">{summary?.total_users ?? "—"}</strong>
            </p>
          </div>
          <div className="mt-4 text-xs text-slate-500 space-y-1">
            <div>• Argon2id + Refresh Token Rotation</div>
            <div>• SSRF Guard + XSS Sanitizer + Bomb Guard</div>
            <div>• Audit Logging đầy đủ trên PostgreSQL</div>
          </div>
        </div>
      </div>
    </div>
  );
}
