import { useCallback, useEffect, useState } from "react";
import { KeyRound, Lock, Shield, Unlock, X } from "lucide-react";
import { apiJson } from "../api/client";
import { useAuth } from "../auth/AuthContext";
import {
  ALL_PERMISSIONS,
  type AdminUserItem,
  type AuditLog,
  type Permission,
  type Role,
} from "../types";

export function AdminPage() {
  const { user, hasPermission } = useAuth();
  const canManageUsers = hasPermission("users:manage");
  const canReadAudit = hasPermission("audit:read");

  const [users, setUsers] = useState<AdminUserItem[]>([]);
  const [logs, setLogs] = useState<AuditLog[]>([]);
  const [editingUser, setEditingUser] = useState<AdminUserItem | null>(null);
  const [overrideMap, setOverrideMap] = useState<Record<string, "default" | "grant" | "revoke">>({});
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      if (canManageUsers) {
        setUsers(await apiJson<AdminUserItem[]>("/api/v1/admin/users"));
      }
      if (canReadAudit) {
        setLogs(await apiJson<AuditLog[]>("/api/v1/admin/audit-logs?limit=60"));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể tải dữ liệu quản trị");
    }
  }, [canManageUsers, canReadAudit]);

  useEffect(() => {
    void load();
  }, [load]);

  const changeRole = async (id: string, role: Role) => {
    setError(null);
    try {
      await apiJson(`/api/v1/admin/users/${id}/role`, {
        method: "PATCH",
        body: JSON.stringify({ role }),
      });
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể đổi vai trò");
    }
  };

  const toggleStatus = async (u: AdminUserItem) => {
    setError(null);
    try {
      await apiJson(`/api/v1/admin/users/${u.id}/status`, {
        method: "PATCH",
        body: JSON.stringify({ is_active: !u.is_active }),
      });
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể cập nhật trạng thái");
    }
  };

  const openPermModal = (u: AdminUserItem) => {
    const map: Record<string, "default" | "grant" | "revoke"> = {};
    for (const p of ALL_PERMISSIONS) {
      const found = u.overrides.find((o) => o.permission === p);
      map[p] = found ? (found.granted ? "grant" : "revoke") : "default";
    }
    setOverrideMap(map);
    setEditingUser(u);
  };

  const savePermissions = async () => {
    if (!editingUser) return;
    setError(null);
    try {
      const overrides = ALL_PERMISSIONS.flatMap((perm: Permission) => {
        const mode = overrideMap[perm];
        if (mode === "grant") return [{ permission: perm, granted: true }];
        if (mode === "revoke") return [{ permission: perm, granted: false }];
        return [];
      });
      await apiJson(`/api/v1/admin/users/${editingUser.id}/permissions`, {
        method: "PUT",
        body: JSON.stringify({ overrides }),
      });
      setEditingUser(null);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Lưu quyền thất bại");
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white">Quản trị Phân quyền Đa tầng &amp; Audit Log</h2>
        <p className="text-sm text-slate-400">
          Phân cấp vai trò (SuperAdmin &gt; Admin &gt; Member &gt; Viewer), ghi đè quyền hạt mịn cho từng người dùng và nhật ký bảo mật.
        </p>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-300">
          {error}
        </div>
      )}

      {canManageUsers && (
        <div className="rounded-2xl border border-slate-800 bg-slate-900/70 overflow-hidden">
          <div className="px-5 py-3.5 border-b border-slate-800 flex items-center gap-2">
            <Shield className="w-4 h-4 text-sky-400" />
            <h3 className="font-semibold text-white text-sm">
              Danh sách tài khoản ({users.length})
            </h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead className="bg-slate-950/60 text-xs uppercase text-slate-400">
                <tr>
                  <th className="px-4 py-3 text-left">Tài khoản</th>
                  <th className="px-4 py-3 text-left">Vai trò</th>
                  <th className="px-4 py-3 text-center">Trạng thái</th>
                  <th className="px-4 py-3 text-center">Override quyền</th>
                  <th className="px-4 py-3 text-right">Thao tác</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {users.map((u) => {
                  const isSelf = u.id === user?.user_id;
                  return (
                    <tr key={u.id} className="hover:bg-slate-800/30">
                      <td className="px-4 py-3">
                        <div className="font-medium text-white">@{u.username}</div>
                        <div className="text-xs text-slate-400">{u.email}</div>
                      </td>
                      <td className="px-4 py-3">
                        <select
                          disabled={isSelf}
                          value={u.role}
                          onChange={(e) => void changeRole(u.id, e.target.value as Role)}
                          className="rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-1 text-xs text-white disabled:opacity-50"
                        >
                          <option value="viewer">Viewer (Tier 1)</option>
                          <option value="member">Member (Tier 2)</option>
                          <option value="admin">Admin (Tier 3)</option>
                          <option value="super_admin">SuperAdmin (Tier 4)</option>
                        </select>
                      </td>
                      <td className="px-4 py-3 text-center">
                        <span
                          className={`px-2 py-0.5 rounded-md text-xs font-medium ${
                            u.is_active
                              ? "bg-emerald-500/15 text-emerald-300"
                              : "bg-rose-500/15 text-rose-300"
                          }`}
                        >
                          {u.is_active ? "Active" : "Locked"}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-center text-xs text-slate-400">
                        {u.overrides.length > 0 ? `${u.overrides.length} quyền riêng` : "Mặc định"}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center justify-end gap-1.5">
                          <button
                            type="button"
                            disabled={isSelf}
                            onClick={() => openPermModal(u)}
                            className="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-40 text-xs text-sky-300 cursor-pointer"
                          >
                            <KeyRound className="w-3.5 h-3.5" /> Quyền
                          </button>
                          <button
                            type="button"
                            disabled={isSelf}
                            onClick={() => void toggleStatus(u)}
                            className="p-1.5 rounded-lg hover:bg-slate-800 disabled:opacity-40 text-slate-300 cursor-pointer"
                            title={u.is_active ? "Khóa tài khoản" : "Mở khóa tài khoản"}
                          >
                            {u.is_active ? (
                              <Lock className="w-4 h-4 text-rose-400" />
                            ) : (
                              <Unlock className="w-4 h-4 text-emerald-400" />
                            )}
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {canReadAudit && (
        <div className="rounded-2xl border border-slate-800 bg-slate-900/70 overflow-hidden">
          <div className="px-5 py-3.5 border-b border-slate-800">
            <h3 className="font-semibold text-white text-sm">
              Nhật ký kiểm toán bảo mật (Audit Logs)
            </h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead className="bg-slate-950/60 uppercase text-slate-400">
                <tr>
                  <th className="px-4 py-2.5 text-left">Thời gian</th>
                  <th className="px-4 py-2.5 text-left">Người thực hiện</th>
                  <th className="px-4 py-2.5 text-left">Hành động</th>
                  <th className="px-4 py-2.5 text-left">Tài nguyên</th>
                  <th className="px-4 py-2.5 text-center">Kết quả</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/70">
                {logs.map((log) => (
                  <tr key={log.id}>
                    <td className="px-4 py-2 font-mono text-slate-400">
                      {new Date(log.created_at).toLocaleString("vi-VN")}
                    </td>
                    <td className="px-4 py-2 text-white">
                      {log.actor_username ? `@${log.actor_username}` : "system"}
                    </td>
                    <td className="px-4 py-2 font-mono text-sky-300">{log.action}</td>
                    <td className="px-4 py-2 text-slate-400">
                      {log.resource_type}
                      {log.resource_id ? ` (${log.resource_id.slice(0, 8)})` : ""}
                    </td>
                    <td className="px-4 py-2 text-center">
                      <span
                        className={`px-2 py-0.5 rounded font-medium ${
                          log.status === "success"
                            ? "bg-emerald-500/15 text-emerald-300"
                            : "bg-rose-500/15 text-rose-300"
                        }`}
                      >
                        {log.status}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {editingUser && (
        <div className="fixed inset-0 z-50 bg-black/75 flex items-center justify-center p-4">
          <div className="rounded-2xl border border-slate-700 bg-slate-900 max-w-lg w-full max-h-[85vh] flex flex-col overflow-hidden">
            <div className="flex items-center justify-between px-5 py-4 border-b border-slate-800">
              <h3 className="font-semibold text-white">
                Ghi đè quyền hạt mịn — @{editingUser.username}
              </h3>
              <button
                type="button"
                onClick={() => setEditingUser(null)}
                className="text-slate-400 hover:text-white cursor-pointer"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            <div className="p-5 overflow-y-auto space-y-2.5">
              {ALL_PERMISSIONS.map((perm) => (
                <div
                  key={perm}
                  className="flex items-center justify-between rounded-xl bg-slate-950 border border-slate-800 px-3 py-2"
                >
                  <span className="font-mono text-xs text-slate-200">{perm}</span>
                  <select
                    value={overrideMap[perm] ?? "default"}
                    onChange={(e) =>
                      setOverrideMap((prev) => ({
                        ...prev,
                        [perm]: e.target.value as "default" | "grant" | "revoke",
                      }))
                    }
                    className="rounded-lg border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-white"
                  >
                    <option value="default">Theo vai trò</option>
                    <option value="grant">Cấp thêm (+)</option>
                    <option value="revoke">Thu hồi (-)</option>
                  </select>
                </div>
              ))}
            </div>
            <div className="px-5 py-3.5 border-t border-slate-800 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setEditingUser(null)}
                className="px-4 py-2 rounded-xl bg-slate-800 text-xs text-slate-300 cursor-pointer"
              >
                Hủy
              </button>
              <button
                type="button"
                onClick={() => void savePermissions()}
                className="px-4 py-2 rounded-xl bg-sky-500 hover:bg-sky-400 text-xs font-semibold text-slate-950 cursor-pointer"
              >
                Lưu phân quyền
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
