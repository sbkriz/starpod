use std::path::Path;

use sqlx::{Executor, SqlitePool};
use tracing::{info, warn};

use starpod_core::{StarpodError, Result};

/// Check whether any legacy database files exist in `db_dir`.
pub fn has_legacy_dbs(db_dir: &Path) -> bool {
    db_dir.join("users.db").exists()
        || db_dir.join("session.db").exists()
        || db_dir.join("cron.db").exists()
}

/// Migrate data from legacy database files into the unified core.db.
///
/// Order matters: users first (FK target), then sessions, then cron.
/// After successful migration, legacy files are renamed to `*.db.migrated`.
///
/// Uses a single connection from the pool for each legacy DB, because
/// `ATTACH DATABASE` is per-connection in SQLite.
pub async fn migrate_legacy_dbs(pool: &SqlitePool, db_dir: &Path) -> Result<()> {
    let users_db = db_dir.join("users.db");
    let session_db = db_dir.join("session.db");
    let cron_db = db_dir.join("cron.db");

    // 1. Migrate users.db (must be first — other tables reference users)
    if users_db.exists() {
        migrate_attached(pool, &users_db, &USERS_QUERIES).await?;
        rename_legacy(&users_db)?;
        info!("Migrated users.db → core.db");
    }

    // 2. Migrate session.db
    if session_db.exists() {
        migrate_attached(pool, &session_db, &SESSION_QUERIES).await?;
        rename_legacy(&session_db)?;
        info!("Migrated session.db → core.db");
    }

    // 3. Migrate cron.db
    if cron_db.exists() {
        migrate_attached(pool, &cron_db, &CRON_QUERIES).await?;
        rename_legacy(&cron_db)?;
        info!("Migrated cron.db → core.db");
    }

    Ok(())
}

/// ATTACH a legacy DB, run a set of INSERT queries, then DETACH — all on one connection.
async fn migrate_attached(pool: &SqlitePool, legacy_path: &Path, queries: &[&str]) -> Result<()> {
    let path_str = legacy_path.display().to_string();
    let mut conn = pool.acquire().await
        .map_err(|e| StarpodError::Database(format!("Failed to acquire connection: {}", e)))?;

    // Temporarily disable FK checks for migration (legacy data may not satisfy new FKs)
    conn.execute("PRAGMA foreign_keys = OFF").await
        .map_err(|e| StarpodError::Database(format!("Failed to disable FK: {}", e)))?;

    conn.execute(format!("ATTACH DATABASE '{}' AS legacy", path_str).as_str()).await
        .map_err(|e| StarpodError::Database(format!("Failed to attach {}: {}", path_str, e)))?;

    for query in queries {
        if let Err(e) = conn.execute(*query).await {
            warn!("Legacy migration query failed (skipping): {}", e);
        }
    }

    conn.execute("DETACH DATABASE legacy").await
        .map_err(|e| StarpodError::Database(format!("Failed to detach {}: {}", path_str, e)))?;

    conn.execute("PRAGMA foreign_keys = ON").await
        .map_err(|e| StarpodError::Database(format!("Failed to re-enable FK: {}", e)))?;

    Ok(())
}

fn rename_legacy(path: &Path) -> Result<()> {
    let mut dest = path.as_os_str().to_os_string();
    dest.push(".migrated");
    std::fs::rename(path, &dest).map_err(|e| {
        StarpodError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to rename {}: {}", path.display(), e),
        ))
    })?;
    // Also rename WAL/SHM files if they exist
    for suffix in &["-wal", "-shm"] {
        let wal = path.with_extension(format!("db{}", suffix));
        if wal.exists() {
            let mut wal_dest = wal.as_os_str().to_os_string();
            wal_dest.push(".migrated");
            let _ = std::fs::rename(&wal, &wal_dest);
        }
    }
    Ok(())
}

// ── Migration queries ──────────────────────────────────────────────────

const USERS_QUERIES: &[&str] = &[
    "INSERT OR IGNORE INTO users (id, email, display_name, role, is_active, created_at, updated_at, filesystem_enabled) \
     SELECT id, email, display_name, role, is_active, created_at, updated_at, filesystem_enabled \
     FROM legacy.users",
    "INSERT OR IGNORE INTO api_keys (id, user_id, prefix, key_hash, label, expires_at, revoked_at, last_used_at, created_at) \
     SELECT id, user_id, prefix, key_hash, label, expires_at, revoked_at, last_used_at, created_at \
     FROM legacy.api_keys",
    "INSERT OR IGNORE INTO telegram_links (telegram_id, user_id, username, linked_at) \
     SELECT telegram_id, user_id, username, linked_at \
     FROM legacy.telegram_links",
    "INSERT OR IGNORE INTO auth_audit_log (id, user_id, event_type, detail, ip_address, created_at) \
     SELECT id, user_id, event_type, detail, ip_address, created_at \
     FROM legacy.auth_audit_log",
];

const SESSION_QUERIES: &[&str] = &[
    "INSERT OR IGNORE INTO session_metadata \
     (id, created_at, last_message_at, is_closed, summary, message_count, channel, channel_session_key, title, user_id, is_read, triggered_by) \
     SELECT id, created_at, last_message_at, is_closed, summary, message_count, \
            COALESCE(channel, 'main'), channel_session_key, title, \
            COALESCE(user_id, 'admin'), COALESCE(is_read, 1), triggered_by \
     FROM legacy.session_metadata",
    "INSERT OR IGNORE INTO session_messages (id, session_id, role, content, timestamp) \
     SELECT id, session_id, role, content, timestamp \
     FROM legacy.session_messages",
    "INSERT OR IGNORE INTO usage_stats \
     (id, session_id, turn, input_tokens, output_tokens, cache_read, cache_write, cost_usd, model, timestamp, user_id) \
     SELECT id, session_id, turn, input_tokens, output_tokens, cache_read, cache_write, cost_usd, model, timestamp, \
            COALESCE(user_id, 'admin') \
     FROM legacy.usage_stats",
    "INSERT OR IGNORE INTO compaction_log (id, session_id, timestamp, trigger, pre_tokens, summary, messages_compacted) \
     SELECT id, session_id, timestamp, trigger, pre_tokens, summary, messages_compacted \
     FROM legacy.compaction_log",
];

const CRON_QUERIES: &[&str] = &[
    "INSERT OR IGNORE INTO cron_jobs \
     (id, name, prompt, schedule_type, schedule_value, enabled, delete_after_run, \
      created_at, last_run_at, next_run_at, retry_count, max_retries, last_error, \
      retry_at, timeout_secs, session_mode, user_id) \
     SELECT id, name, prompt, schedule_type, schedule_value, enabled, delete_after_run, \
            created_at, last_run_at, next_run_at, \
            COALESCE(retry_count, 0), COALESCE(max_retries, 3), last_error, \
            retry_at, COALESCE(timeout_secs, 7200), COALESCE(session_mode, 'isolated'), user_id \
     FROM legacy.cron_jobs",
    "INSERT OR IGNORE INTO cron_runs (id, job_id, started_at, completed_at, status, result_summary, session_id) \
     SELECT id, job_id, started_at, completed_at, status, result_summary, session_id \
     FROM legacy.cron_runs",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoreDb;

    #[tokio::test]
    async fn no_legacy_dbs_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!has_legacy_dbs(tmp.path()));
    }

    #[tokio::test]
    async fn has_legacy_detects_each_file() {
        let tmp = tempfile::tempdir().unwrap();
        let db_dir = tmp.path();

        // None present
        assert!(!has_legacy_dbs(db_dir));

        // users.db only
        std::fs::write(db_dir.join("users.db"), b"").unwrap();
        assert!(has_legacy_dbs(db_dir));
        std::fs::remove_file(db_dir.join("users.db")).unwrap();

        // session.db only
        std::fs::write(db_dir.join("session.db"), b"").unwrap();
        assert!(has_legacy_dbs(db_dir));
        std::fs::remove_file(db_dir.join("session.db")).unwrap();

        // cron.db only
        std::fs::write(db_dir.join("cron.db"), b"").unwrap();
        assert!(has_legacy_dbs(db_dir));
    }

    /// Helper: create a legacy SQLite DB with the given schema + data statements.
    async fn create_legacy_db(path: &std::path::Path, statements: &[&str]) {
        let pool = SqlitePool::connect(
            &format!("sqlite://{}?mode=rwc", path.display())
        ).await.unwrap();
        for stmt in statements {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        pool.close().await;
    }

    #[tokio::test]
    async fn migrate_users_db() {
        let tmp = tempfile::tempdir().unwrap();
        let db_dir = tmp.path();

        create_legacy_db(&db_dir.join("users.db"), &[
            "CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT, display_name TEXT, \
             role TEXT NOT NULL DEFAULT 'user', is_active INTEGER NOT NULL DEFAULT 1, \
             created_at TEXT NOT NULL, updated_at TEXT NOT NULL, filesystem_enabled INTEGER NOT NULL DEFAULT 0)",
            "INSERT INTO users (id, email, display_name, role, created_at, updated_at) \
             VALUES ('u1', 'test@test.com', 'Test', 'admin', '2024-01-01', '2024-01-01')",
        ]).await;

        let db = CoreDb::new(db_dir).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT email FROM users WHERE id = 'u1'")
            .fetch_one(db.pool()).await.unwrap();
        assert_eq!(row.0, "test@test.com");

        assert!(!db_dir.join("users.db").exists());
        assert!(db_dir.join("users.db.migrated").exists());
    }

    #[tokio::test]
    async fn migrate_session_db() {
        let tmp = tempfile::tempdir().unwrap();
        let db_dir = tmp.path();

        create_legacy_db(&db_dir.join("session.db"), &[
            "CREATE TABLE session_metadata (id TEXT PRIMARY KEY, created_at TEXT NOT NULL, \
             last_message_at TEXT NOT NULL, is_closed INTEGER NOT NULL DEFAULT 0, \
             summary TEXT, message_count INTEGER NOT NULL DEFAULT 0, \
             channel TEXT DEFAULT 'main', channel_session_key TEXT, title TEXT, \
             user_id TEXT DEFAULT 'admin', is_read INTEGER DEFAULT 1, triggered_by TEXT)",
            "INSERT INTO session_metadata (id, created_at, last_message_at, title) \
             VALUES ('s1', '2024-01-01', '2024-01-02', 'Test Session')",
            "CREATE TABLE session_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, \
             session_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL, \
             timestamp TEXT NOT NULL)",
            "INSERT INTO session_messages (session_id, role, content, timestamp) \
             VALUES ('s1', 'user', 'hello world', '2024-01-01')",
        ]).await;

        let db = CoreDb::new(db_dir).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT title FROM session_metadata WHERE id = 's1'")
            .fetch_one(db.pool()).await.unwrap();
        assert_eq!(row.0, "Test Session");

        let row: (String,) = sqlx::query_as(
            "SELECT content FROM session_messages WHERE session_id = 's1'"
        ).fetch_one(db.pool()).await.unwrap();
        assert_eq!(row.0, "hello world");

        assert!(!db_dir.join("session.db").exists());
        assert!(db_dir.join("session.db.migrated").exists());
    }

    #[tokio::test]
    async fn migrate_cron_db() {
        let tmp = tempfile::tempdir().unwrap();
        let db_dir = tmp.path();

        create_legacy_db(&db_dir.join("cron.db"), &[
            "CREATE TABLE cron_jobs (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, \
             prompt TEXT NOT NULL, schedule_type TEXT NOT NULL, schedule_value TEXT NOT NULL, \
             enabled INTEGER NOT NULL DEFAULT 1, delete_after_run INTEGER NOT NULL DEFAULT 0, \
             created_at INTEGER NOT NULL, last_run_at INTEGER, next_run_at INTEGER, \
             retry_count INTEGER DEFAULT 0, max_retries INTEGER DEFAULT 3, last_error TEXT, \
             retry_at INTEGER, timeout_secs INTEGER DEFAULT 7200, \
             session_mode TEXT DEFAULT 'isolated', user_id TEXT)",
            "INSERT INTO cron_jobs (id, name, prompt, schedule_type, schedule_value, created_at) \
             VALUES ('j1', 'daily-check', 'run checks', 'cron', '0 9 * * *', 1000)",
            "CREATE TABLE cron_runs (id TEXT PRIMARY KEY, job_id TEXT NOT NULL, \
             started_at INTEGER NOT NULL, completed_at INTEGER, status TEXT NOT NULL DEFAULT 'pending', \
             result_summary TEXT, session_id TEXT)",
            "INSERT INTO cron_runs (id, job_id, started_at, status) \
             VALUES ('r1', 'j1', 2000, 'success')",
        ]).await;

        let db = CoreDb::new(db_dir).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT prompt FROM cron_jobs WHERE id = 'j1'")
            .fetch_one(db.pool()).await.unwrap();
        assert_eq!(row.0, "run checks");

        let row: (String,) = sqlx::query_as("SELECT status FROM cron_runs WHERE id = 'r1'")
            .fetch_one(db.pool()).await.unwrap();
        assert_eq!(row.0, "success");

        assert!(!db_dir.join("cron.db").exists());
        assert!(db_dir.join("cron.db.migrated").exists());
    }

    #[tokio::test]
    async fn migrate_all_three_legacy_dbs() {
        let tmp = tempfile::tempdir().unwrap();
        let db_dir = tmp.path();

        // Create all three legacy DBs
        create_legacy_db(&db_dir.join("users.db"), &[
            "CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT, display_name TEXT, \
             role TEXT NOT NULL DEFAULT 'user', is_active INTEGER NOT NULL DEFAULT 1, \
             created_at TEXT NOT NULL, updated_at TEXT NOT NULL, filesystem_enabled INTEGER NOT NULL DEFAULT 0)",
            "INSERT INTO users (id, role, created_at, updated_at) \
             VALUES ('u1', 'admin', '2024-01-01', '2024-01-01')",
        ]).await;

        create_legacy_db(&db_dir.join("session.db"), &[
            "CREATE TABLE session_metadata (id TEXT PRIMARY KEY, created_at TEXT NOT NULL, \
             last_message_at TEXT NOT NULL, is_closed INTEGER NOT NULL DEFAULT 0, \
             summary TEXT, message_count INTEGER NOT NULL DEFAULT 0, \
             channel TEXT DEFAULT 'main', channel_session_key TEXT, title TEXT, \
             user_id TEXT DEFAULT 'admin', is_read INTEGER DEFAULT 1, triggered_by TEXT)",
            "INSERT INTO session_metadata (id, created_at, last_message_at) \
             VALUES ('s1', '2024-01-01', '2024-01-01')",
        ]).await;

        create_legacy_db(&db_dir.join("cron.db"), &[
            "CREATE TABLE cron_jobs (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, \
             prompt TEXT NOT NULL, schedule_type TEXT NOT NULL, schedule_value TEXT NOT NULL, \
             enabled INTEGER NOT NULL DEFAULT 1, delete_after_run INTEGER NOT NULL DEFAULT 0, \
             created_at INTEGER NOT NULL, last_run_at INTEGER, next_run_at INTEGER, \
             retry_count INTEGER DEFAULT 0, max_retries INTEGER DEFAULT 3, last_error TEXT, \
             retry_at INTEGER, timeout_secs INTEGER DEFAULT 7200, \
             session_mode TEXT DEFAULT 'isolated', user_id TEXT)",
            "INSERT INTO cron_jobs (id, name, prompt, schedule_type, schedule_value, created_at) \
             VALUES ('j1', 'test', 'prompt', 'interval', '60000', 1000)",
        ]).await;

        let db = CoreDb::new(db_dir).await.unwrap();

        // All data should be present
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(db.pool()).await.unwrap();
        assert_eq!(row.0, 1);

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM session_metadata")
            .fetch_one(db.pool()).await.unwrap();
        assert_eq!(row.0, 1);

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cron_jobs")
            .fetch_one(db.pool()).await.unwrap();
        assert_eq!(row.0, 1);

        // All legacy files renamed
        for name in &["users.db", "session.db", "cron.db"] {
            assert!(!db_dir.join(name).exists(), "{} should be gone", name);
            assert!(db_dir.join(format!("{}.migrated", name)).exists(),
                "{}.migrated should exist", name);
        }
    }

    #[tokio::test]
    async fn second_open_skips_migration() {
        let tmp = tempfile::tempdir().unwrap();
        let db_dir = tmp.path();

        create_legacy_db(&db_dir.join("users.db"), &[
            "CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT, display_name TEXT, \
             role TEXT NOT NULL DEFAULT 'user', is_active INTEGER NOT NULL DEFAULT 1, \
             created_at TEXT NOT NULL, updated_at TEXT NOT NULL, filesystem_enabled INTEGER NOT NULL DEFAULT 0)",
            "INSERT INTO users (id, role, created_at, updated_at) \
             VALUES ('u1', 'admin', '2024-01-01', '2024-01-01')",
        ]).await;

        // First open — migrates
        let db1 = CoreDb::new(db_dir).await.unwrap();
        assert!(!db_dir.join("users.db").exists());
        drop(db1);

        // Second open — no legacy files, should not fail
        let db2 = CoreDb::new(db_dir).await.unwrap();
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(db2.pool()).await.unwrap();
        assert_eq!(row.0, 1);
    }
}
