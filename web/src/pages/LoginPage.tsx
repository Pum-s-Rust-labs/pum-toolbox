import { useState, type FormEvent } from "react";
import { ShieldCheck, Lock, UserPlus, LogIn } from "lucide-react";
import { useAuth } from "../auth/AuthContext";

export function LoginPage() {
  const { login, register } = useAuth();
  const [mode, setMode] = useState<"login" | "register">("login");
  const [identifier, setIdentifier] = useState("");
  const [email, setEmail] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      if (mode === "login") {
        await login(identifier, password);
      } else {
        await register(email, username, password);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Thao tác thất bại");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-sky-950/60 via-slate-950 to-slate-950">
      <div className="w-full max-w-md rounded-2xl border border-slate-800 bg-slate-900/80 backdrop-blur-xl p-8 shadow-2xl">
        <div className="flex items-center gap-3 mb-6">
          <div className="h-11 w-11 rounded-xl bg-sky-500/15 border border-sky-400/30 flex items-center justify-center text-sky-400">
            <ShieldCheck className="w-6 h-6" />
          </div>
          <div>
            <h1 className="text-xl font-bold text-white tracking-tight">Pum Toolbox</h1>
            <p className="text-xs text-slate-400">All-in-One Rust Dev & Ops Platform</p>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-1 p-1 rounded-xl bg-slate-950/80 border border-slate-800 mb-6">
          <button
            type="button"
            onClick={() => {
              setMode("login");
              setError(null);
            }}
            className={`flex items-center justify-center gap-1.5 py-2 rounded-lg text-sm font-medium transition ${
              mode === "login"
                ? "bg-sky-500 text-slate-950 shadow"
                : "text-slate-400 hover:text-white"
            }`}
          >
            <LogIn className="w-4 h-4" /> Đăng nhập
          </button>
          <button
            type="button"
            onClick={() => {
              setMode("register");
              setError(null);
            }}
            className={`flex items-center justify-center gap-1.5 py-2 rounded-lg text-sm font-medium transition ${
              mode === "register"
                ? "bg-sky-500 text-slate-950 shadow"
                : "text-slate-400 hover:text-white"
            }`}
          >
            <UserPlus className="w-4 h-4" /> Đăng ký
          </button>
        </div>

        {error && (
          <div className="mb-4 rounded-xl border border-rose-500/30 bg-rose-500/10 px-3.5 py-2.5 text-sm text-rose-300">
            {error}
          </div>
        )}

        <form onSubmit={onSubmit} className="space-y-4">
          {mode === "login" ? (
            <div>
              <label className="block text-xs font-medium text-slate-300 mb-1.5">
                Email hoặc Username
              </label>
              <input
                type="text"
                required
                value={identifier}
                onChange={(e) => setIdentifier(e.target.value)}
                placeholder="superadmin hoặc admin@pumlabs.dev"
                className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white placeholder-slate-500 focus:border-sky-500 focus:outline-none"
              />
            </div>
          ) : (
            <>
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1.5">Email</label>
                <input
                  type="email"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="you@pumlabs.dev"
                  className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white placeholder-slate-500 focus:border-sky-500 focus:outline-none"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1.5">Username</label>
                <input
                  type="text"
                  required
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="pum_dev"
                  className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white placeholder-slate-500 focus:border-sky-500 focus:outline-none"
                />
              </div>
            </>
          )}

          <div>
            <label className="block text-xs font-medium text-slate-300 mb-1.5">Mật khẩu</label>
            <input
              type="password"
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Ít nhất 8 ký tự (Hoa + thường + số)"
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3.5 py-2.5 text-sm text-white placeholder-slate-500 focus:border-sky-500 focus:outline-none"
            />
          </div>

          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded-xl bg-sky-500 hover:bg-sky-400 disabled:opacity-50 text-slate-950 font-semibold py-2.5 text-sm transition flex items-center justify-center gap-2 cursor-pointer"
          >
            <Lock className="w-4 h-4" />
            {submitting
              ? "Đang xác thực..."
              : mode === "login"
                ? "Đăng nhập an toàn"
                : "Tạo tài khoản"}
          </button>
        </form>

        <div className="mt-6 pt-4 border-t border-slate-800 text-xs text-slate-500 space-y-1">
          <p>Bảo vệ bởi Argon2id · JWT + HttpOnly Refresh Rotation · RBAC 3 tầng</p>
          <p>Pum&apos;s Rust Labs — Powered by Rust (Axum + Tokio + PostgreSQL)</p>
        </div>
      </div>
    </div>
  );
}
