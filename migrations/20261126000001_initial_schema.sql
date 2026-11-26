-- Pum Toolbox Initial PostgreSQL Schema
-- Covers: Users, Multi-tier RBAC, Refresh Token Rotation, Audit Logs,
--         Shortlinks + Analytics, Uptime Monitors + Checks, Pastes, Image Jobs.

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('super_admin', 'admin', 'member', 'viewer')),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_permissions (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission TEXT NOT NULL,
    granted BOOLEAN NOT NULL,
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, permission)
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id UUID NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    user_agent TEXT,
    ip_address TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family_id ON refresh_tokens(family_id);

CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY,
    actor_id UUID REFERENCES users(id) ON DELETE SET NULL,
    actor_username TEXT,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    status TEXT NOT NULL DEFAULT 'success',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_id ON audit_logs(actor_id);

-- Module 1: Shortlink & Analytics
CREATE TABLE IF NOT EXISTS short_links (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    slug TEXT NOT NULL UNIQUE,
    target_url TEXT NOT NULL,
    title TEXT,
    visibility TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('private', 'internal', 'public')),
    password_hash TEXT,
    expires_at TIMESTAMPTZ,
    click_count BIGINT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_short_links_owner_id ON short_links(owner_id);
CREATE INDEX IF NOT EXISTS idx_short_links_created_at ON short_links(created_at DESC);

CREATE TABLE IF NOT EXISTS link_clicks (
    id UUID PRIMARY KEY,
    link_id UUID NOT NULL REFERENCES short_links(id) ON DELETE CASCADE,
    referer TEXT,
    user_agent TEXT,
    device_type TEXT NOT NULL DEFAULT 'desktop',
    ip_hash TEXT,
    clicked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_link_clicks_link_id_clicked_at ON link_clicks(link_id, clicked_at DESC);

-- Module 2: Website / API Uptime Monitor
CREATE TABLE IF NOT EXISTS monitors (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    method TEXT NOT NULL DEFAULT 'GET' CHECK (method IN ('GET', 'HEAD', 'POST')),
    expected_status INTEGER NOT NULL DEFAULT 200,
    interval_secs INTEGER NOT NULL DEFAULT 60 CHECK (interval_secs >= 15 AND interval_secs <= 3600),
    timeout_ms INTEGER NOT NULL DEFAULT 5000 CHECK (timeout_ms >= 500 AND timeout_ms <= 30000),
    visibility TEXT NOT NULL DEFAULT 'internal' CHECK (visibility IN ('private', 'internal', 'public')),
    is_paused BOOLEAN NOT NULL DEFAULT FALSE,
    last_status TEXT NOT NULL DEFAULT 'pending' CHECK (last_status IN ('pending', 'up', 'down')),
    last_status_code INTEGER,
    last_latency_ms INTEGER,
    last_error TEXT,
    last_checked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_monitors_owner_id ON monitors(owner_id);

CREATE TABLE IF NOT EXISTS monitor_checks (
    id UUID PRIMARY KEY,
    monitor_id UUID NOT NULL REFERENCES monitors(id) ON DELETE CASCADE,
    is_up BOOLEAN NOT NULL,
    status_code INTEGER,
    latency_ms INTEGER NOT NULL,
    error_message TEXT,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_monitor_checks_monitor_checked ON monitor_checks(monitor_id, checked_at DESC);

-- Module 3: Code / Markdown Pastebin
CREATE TABLE IF NOT EXISTS pastes (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    rendered_html TEXT,
    language TEXT NOT NULL DEFAULT 'markdown',
    tags TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    visibility TEXT NOT NULL DEFAULT 'internal' CHECK (visibility IN ('private', 'internal', 'public')),
    max_views INTEGER CHECK (max_views IS NULL OR max_views > 0),
    view_count INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pastes_owner_id ON pastes(owner_id);
CREATE INDEX IF NOT EXISTS idx_pastes_created_at ON pastes(created_at DESC);

-- Module 4: Image Processing Jobs History
CREATE TABLE IF NOT EXISTS image_jobs (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    file_count INTEGER NOT NULL,
    output_format TEXT NOT NULL,
    quality INTEGER NOT NULL,
    original_bytes BIGINT NOT NULL,
    processed_bytes BIGINT NOT NULL,
    duration_ms INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_image_jobs_owner_id ON image_jobs(owner_id, created_at DESC);
