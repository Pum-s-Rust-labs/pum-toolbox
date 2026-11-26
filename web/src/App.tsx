import { useState } from "react";
import {
  Activity,
  FileCode2,
  Image as ImageIcon,
  LayoutDashboard,
  Link2,
  LogOut,
  Shield,
  ShieldCheck,
} from "lucide-react";
import { AuthProvider, useAuth } from "./auth/AuthContext";
import { AdminPage } from "./pages/AdminPage";
import { DashboardPage } from "./pages/DashboardPage";
import { ImageLabPage } from "./pages/ImageLabPage";
import { LinksPage } from "./pages/LinksPage";
import { LoginPage } from "./pages/LoginPage";
import { MonitorsPage } from "./pages/MonitorsPage";
import { PastesPage } from "./pages/PastesPage";

function AppShell() {
  const { user, loading, logout, hasPermission } = useAuth();
  const [tab, setTab] = useState("dashboard");

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-slate-950 text-slate-400 text-sm">
        Đang xác thực phiên Pum Toolbox...
      </div>
    );
  }

  if (!user) {
    return <LoginPage />;
  }

  const canSeeAdmin = hasPermission("users:manage") || hasPermission("audit:read");

  const navItems = [
    { id: "dashboard", label: "Tổng quan", icon: LayoutDashboard, show: true },
    { id: "links", label: "Shortlinks & QR", icon: Link2, show: hasPermission("links:read") },
    { id: "monitors", label: "Uptime Monitor", icon: Activity, show: hasPermission("monitors:read") },
    { id: "pastes", label: "Code & Markdown", icon: FileCode2, show: hasPermission("pastes:read") },
    { id: "images", label: "ImageLab", icon: ImageIcon, show: true },
    { id: "admin", label: "RBAC & Audit Log", icon: Shield, show: canSeeAdmin },
  ].filter((item) => item.show);

  return (
    <div className="min-h-screen flex bg-slate-950 text-slate-100">
      <aside className="w-64 shrink-0 border-r border-slate-800/80 bg-slate-900/50 hidden md:flex flex-col justify-between p-4">
        <div className="space-y-6">
          <div className="flex items-center gap-3 px-2 pt-1">
            <div className="h-9 w-9 rounded-xl bg-sky-500/15 border border-sky-400/30 flex items-center justify-center text-sky-400">
              <ShieldCheck className="w-5 h-5" />
            </div>
            <div>
              <div className="font-bold text-white leading-tight">Pum Toolbox</div>
              <div className="text-[11px] text-slate-400">Pum&apos;s Rust Labs</div>
            </div>
          </div>

          <nav className="space-y-1">
            {navItems.map((item) => {
              const Icon = item.icon;
              const active = tab === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setTab(item.id)}
                  className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition cursor-pointer ${
                    active
                      ? "bg-sky-500/15 text-sky-300 border border-sky-500/25"
                      : "text-slate-400 hover:text-white hover:bg-slate-800/50 border border-transparent"
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  {item.label}
                </button>
              );
            })}
          </nav>
        </div>

        <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-3 space-y-2.5">
          <div>
            <div className="text-sm font-semibold text-white truncate">@{user.username}</div>
            <div className="text-xs text-slate-400 truncate">{user.email}</div>
            <span className="inline-block mt-1 px-2 py-0.5 rounded bg-sky-500/15 text-sky-300 text-[10px] font-semibold uppercase tracking-wide">
              {user.role}
            </span>
          </div>
          <button
            type="button"
            onClick={() => void logout()}
            className="w-full flex items-center justify-center gap-2 py-1.5 rounded-lg bg-slate-800 hover:bg-rose-500/20 hover:text-rose-300 text-xs text-slate-300 transition cursor-pointer"
          >
            <LogOut className="w-3.5 h-3.5" /> Đăng xuất
          </button>
        </div>
      </aside>

      <div className="flex-1 flex flex-col min-w-0">
        <header className="md:hidden flex items-center justify-between px-4 py-3 border-b border-slate-800 bg-slate-900/70">
          <span className="font-bold text-white">Pum Toolbox</span>
          <div className="flex items-center gap-2">
            <select
              value={tab}
              onChange={(e) => setTab(e.target.value)}
              className="rounded-lg border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-white"
            >
              {navItems.map((n) => (
                <option key={n.id} value={n.id}>
                  {n.label}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={() => void logout()}
              className="p-1.5 rounded-lg bg-slate-800 text-slate-300"
            >
              <LogOut className="w-4 h-4" />
            </button>
          </div>
        </header>

        <main className="flex-1 p-5 md:p-8 max-w-6xl w-full mx-auto">
          {tab === "dashboard" && <DashboardPage onNavigate={setTab} />}
          {tab === "links" && <LinksPage />}
          {tab === "monitors" && <MonitorsPage />}
          {tab === "pastes" && <PastesPage />}
          {tab === "images" && <ImageLabPage />}
          {tab === "admin" && canSeeAdmin && <AdminPage />}
        </main>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <AuthProvider>
      <AppShell />
    </AuthProvider>
  );
}
