-- Auth tables: users, api_keys, telegram_links, audit log

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE,
    display_name TEXT,
    role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('admin', 'user')),
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    filesystem_enabled INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    label TEXT,
    expires_at TEXT,
    revoked_at TEXT,
    last_used_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(prefix);
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);

CREATE TABLE IF NOT EXISTS telegram_links (
    telegram_id INTEGER PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username TEXT,
    linked_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_telegram_links_user_id ON telegram_links(user_id);

CREATE TABLE IF NOT EXISTS auth_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT,
    event_type TEXT NOT NULL,
    detail TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_auth_audit_log_user_id ON auth_audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_audit_log_created_at ON auth_audit_log(created_at);

-- Session tables: metadata, messages, usage stats, compaction log

CREATE TABLE IF NOT EXISTS session_metadata (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    last_message_at TEXT NOT NULL,
    is_closed INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    channel TEXT NOT NULL DEFAULT 'main',
    channel_session_key TEXT,
    title TEXT,
    user_id TEXT NOT NULL DEFAULT 'admin',
    is_read INTEGER NOT NULL DEFAULT 1,
    triggered_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_session_channel_key
    ON session_metadata(channel, channel_session_key, is_closed, last_message_at DESC);

CREATE INDEX IF NOT EXISTS idx_session_user_channel
    ON session_metadata(user_id, channel, channel_session_key);

CREATE TABLE IF NOT EXISTS session_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES session_metadata(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_session_messages_session
    ON session_messages(session_id, id);

CREATE TABLE IF NOT EXISTS usage_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    turn INTEGER NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cache_read INTEGER NOT NULL DEFAULT 0,
    cache_write INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    model TEXT,
    timestamp TEXT NOT NULL,
    user_id TEXT NOT NULL DEFAULT 'admin'
);

CREATE INDEX IF NOT EXISTS idx_usage_stats_user
    ON usage_stats(user_id, timestamp);

CREATE TABLE IF NOT EXISTS compaction_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES session_metadata(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL,
    trigger TEXT NOT NULL DEFAULT 'auto',
    pre_tokens INTEGER NOT NULL,
    summary TEXT NOT NULL,
    messages_compacted INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_compaction_log_session
    ON compaction_log(session_id);

-- Cron tables: jobs and run history

CREATE TABLE IF NOT EXISTS cron_jobs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    prompt TEXT NOT NULL,
    schedule_type TEXT NOT NULL,
    schedule_value TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    delete_after_run INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    last_run_at INTEGER,
    next_run_at INTEGER,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    last_error TEXT,
    retry_at INTEGER,
    timeout_secs INTEGER NOT NULL DEFAULT 7200,
    session_mode TEXT NOT NULL DEFAULT 'isolated',
    user_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_cron_jobs_next
    ON cron_jobs(next_run_at);

CREATE TABLE IF NOT EXISTS cron_runs (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES cron_jobs(id) ON DELETE CASCADE,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL DEFAULT 'pending',
    result_summary TEXT,
    session_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_cron_runs_job
    ON cron_runs(job_id, started_at DESC);
