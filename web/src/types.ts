export type Role = "super_admin" | "admin" | "member" | "viewer";
export type Visibility = "private" | "internal" | "public";

export const ALL_PERMISSIONS = [
  "links:read",
  "links:write",
  "links:delete_any",
  "monitors:read",
  "monitors:write",
  "monitors:delete_any",
  "pastes:read",
  "pastes:write",
  "pastes:delete_any",
  "images:process",
  "users:manage",
  "roles:assign",
  "audit:read",
] as const;

export type Permission = (typeof ALL_PERMISSIONS)[number];

export interface Principal {
  user_id: string;
  email: string;
  username: string;
  role: Role;
  permissions: Permission[];
}

export interface AuthResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
  user: Principal;
}

export interface DashboardSummary {
  total_links: number;
  total_clicks: number;
  total_monitors: number;
  monitors_up: number;
  monitors_down: number;
  total_pastes: number;
  total_image_jobs: number;
  total_bytes_saved: number;
  total_users: number;
}

export interface ShortLink {
  id: string;
  owner_id: string;
  slug: string;
  target_url: string;
  title: string | null;
  visibility: Visibility;
  has_password: boolean;
  expires_at: string | null;
  click_count: number;
  is_active: boolean;
  created_at: string;
}

export interface LinkAnalytics {
  link: ShortLink;
  short_url: string;
  daily: { day: string; clicks: number }[];
  devices: { device_type: string; clicks: number }[];
}

export interface Monitor {
  id: string;
  owner_id: string;
  name: string;
  url: string;
  method: string;
  expected_status: number;
  interval_secs: number;
  timeout_ms: number;
  visibility: Visibility;
  is_paused: boolean;
  last_status: "pending" | "up" | "down";
  last_status_code: number | null;
  last_latency_ms: number | null;
  last_error: string | null;
  last_checked_at: string | null;
  created_at: string;
}

export interface MonitorCheck {
  id: string;
  monitor_id: string;
  is_up: boolean;
  status_code: number | null;
  latency_ms: number;
  error_message: string | null;
  checked_at: string;
}

export interface MonitorEvent {
  monitor_id: string;
  name: string;
  url: string;
  is_up: boolean;
  status_code: number | null;
  latency_ms: number;
  error_message: string | null;
  checked_at: string;
}

export interface PasteSummary {
  id: string;
  owner_id: string;
  slug: string;
  title: string;
  language: string;
  tags: string[];
  visibility: Visibility;
  max_views: number | null;
  view_count: number;
  expires_at: string | null;
  created_at: string;
}

export interface PasteDetail extends PasteSummary {
  content: string;
  rendered_html: string | null;
}

export interface ImageJob {
  id: string;
  owner_id: string;
  file_count: number;
  output_format: string;
  quality: number;
  original_bytes: number;
  processed_bytes: number;
  duration_ms: number;
  created_at: string;
}

export interface PermissionOverride {
  user_id?: string;
  permission: Permission;
  granted: boolean;
}

export interface AdminUserItem {
  id: string;
  email: string;
  username: string;
  role: Role;
  is_active: boolean;
  locked_until: string | null;
  created_at: string;
  overrides: PermissionOverride[];
}

export interface AuditLog {
  id: string;
  actor_id: string | null;
  actor_username: string | null;
  action: string;
  resource_type: string;
  resource_id: string | null;
  status: string;
  metadata: Record<string, unknown>;
  ip_address: string | null;
  user_agent: string | null;
  created_at: string;
}
