# starpod-vault

AES-256-GCM encrypted credential storage in SQLite with audit logging.

## API

```rust
let vault = Vault::new(&db_path, &master_key).await?;

// Store a credential
vault.set("github_token", "ghp_xxx", Some("user_123")).await?;

// Store with metadata (is_secret, allowed_hosts)
vault.set_with_meta(
    "OPENAI_API_KEY",
    "sk-xxx",
    true,                                  // is_secret
    Some(&["api.openai.com".into()]),      // allowed_hosts
    Some("user_123"),
).await?;

// Retrieve (decrypted)
let value = vault.get("github_token", Some("user_123")).await?; // Option<String>

// Retrieve metadata (no decryption)
let entry = vault.get_entry("OPENAI_API_KEY").await?; // Option<VaultEntry>

// List all keys
let keys = vault.list_keys().await?; // Vec<String>

// List all entries with metadata
let entries = vault.list_entries().await?; // Vec<VaultEntry>

// Update metadata without re-encrypting the value
vault.update_meta("OPENAI_API_KEY", true, Some(&["api.openai.com".into()])).await?;

// Delete
vault.delete("github_token", None).await?;
```

## VaultEntry

Metadata returned by `get_entry()` and `list_entries()` — no decrypted values.

| Field | Type | Description |
|-------|------|-------------|
| `key` | `String` | Entry name |
| `is_secret` | `bool` | `true` (default) = opaque-ified when proxy enabled |
| `allowed_hosts` | `Option<Vec<String>>` | Hostnames where secret may be sent; `None` = unrestricted |
| `created_at` | `String` | RFC 3339 timestamp |
| `updated_at` | `String` | RFC 3339 timestamp |

## Opaque Tokens (feature: `secret-proxy`)

When the `secret-proxy` feature is enabled, the crate provides opaque token encode/decode:

```rust
// Encode: value + allowed hosts → opaque token
let token = starpod_vault::opaque::encode_opaque_token(
    vault.cipher(),
    "ghp_abc123",
    &["api.github.com".into()],
)?;
// token = "starpod:v1:<base64(nonce ++ ciphertext)>"

// Decode: opaque token → (value, allowed hosts)
let (value, hosts) = starpod_vault::opaque::decode_opaque_token(
    vault.cipher(),
    &token,
)?;

// Check if a string is an opaque token
assert!(starpod_vault::opaque::is_opaque_token(&token));
```

The encrypted payload is `{"v": "<real_value>", "h": ["host1", ...]}`, encrypted with the same AES-256-GCM key used for at-rest vault encryption. Each token uses a fresh random nonce, so the same secret produces different tokens each time.

## Known Hosts

`known_hosts::default_hosts_for_key()` returns auto-suggested allowed hosts for well-known keys:

```rust
use starpod_vault::known_hosts::default_hosts_for_key;

assert_eq!(
    default_hosts_for_key("OPENAI_API_KEY"),
    Some(vec!["api.openai.com".into()]),
);
assert!(default_hosts_for_key("MY_CUSTOM_VAR").is_none());
```

## System Keys

`SYSTEM_KEYS` is a centralized list of environment variable names that hold
system-managed secrets (LLM provider keys, service tokens, platform secrets).
The `is_system_key(key)` helper performs a case-insensitive check against this
list.

System keys are protected at two layers:
- The `EnvGet` and `VaultGet` agent tools use `is_system_key()` to block reads.
- The `ToolExecutor` Bash runner uses `env_blocklist` (populated from `SYSTEM_KEYS`) to strip them from child process environments via `env_remove()`.

| Category | Keys |
|----------|------|
| LLM providers | `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `GROQ_API_KEY`, `DEEPSEEK_API_KEY`, `OPENROUTER_API_KEY` |
| Services | `BRAVE_API_KEY`, `TELEGRAM_BOT_TOKEN` |
| Platform | `STARPOD_API_KEY` |

## Encryption

- **Algorithm**: AES-256-GCM (128-bit auth tag, 96-bit nonce)
- **Master key**: 32-byte random key stored at `.starpod/db/.vault_key`
- **At-rest**: Values encrypted in `vault_entries.encrypted_value` column
- **Runtime (proxy)**: Opaque tokens use the same key, with a separate nonce per token
- **Storage**: SQLite database (WAL mode)
- **Audit**: All get/set/delete/update_meta operations logged with optional `user_id`

## Schema

```sql
-- 001_init.sql
CREATE TABLE vault_entries (
    key TEXT PRIMARY KEY,
    encrypted_value BLOB NOT NULL,
    nonce BLOB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE vault_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL,
    action TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    user_id TEXT
);

-- 002_secret_proxy.sql
ALTER TABLE vault_entries ADD COLUMN is_secret INTEGER NOT NULL DEFAULT 1;
ALTER TABLE vault_entries ADD COLUMN allowed_hosts TEXT;
```

## Feature Flags

| Feature | Dependencies | What it enables |
|---------|-------------|-----------------|
| `secret-proxy` | `base64` | `opaque` module (encode/decode opaque tokens), `Vault::cipher()` accessor |

## Tests

28 unit tests + 8 opaque token tests + 7 known hosts tests + 2 doc-tests.
