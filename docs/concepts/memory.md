# Memory

Starpod's memory system combines **markdown files on disk** with a **SQLite FTS5 full-text search index**. The agent can read, write, and search memory — and context is automatically bootstrapped into every conversation.

## File Layout

```
.starpod/
├── SOUL.md          Agent personality and instructions
├── HEARTBEAT.md     Heartbeat task instructions (periodic proactive behavior)
├── BOOT.md          Boot lifecycle prompt (runs every server start)
├── BOOTSTRAP.md     First-init bootstrap (runs once, then cleared)
├── db/
│   └── memory.db    SQLite FTS5 + vector index
├── users/<id>/      Per-user files
│   ├── USER.md      User information and preferences
│   ├── MEMORY.md    General long-term knowledge
│   └── memory/      Daily conversation logs
│       ├── 2026-03-12.md
│       ├── 2026-03-13.md
│       └── 2026-03-14.md
└── skills/          Agent skills (see Skills docs)
```

### Agent-Level Files

| File | Purpose | Auto-loaded? |
|------|---------|:---:|
| `SOUL.md` | Agent personality, instructions, behavioral guidelines | Yes |
| `HEARTBEAT.md` | Heartbeat task instructions | No (read by scheduler) |
| `BOOT.md` | Boot lifecycle prompt | No (runs at startup) |
| `BOOTSTRAP.md` | First-init bootstrap prompt (self-destructing) | No (runs once) |

### Per-User Files

Each user gets their own directory at `.starpod/users/<id>/`:

| File | Purpose | Auto-loaded? |
|------|---------|:---:|
| `USER.md` | User info — name, role, preferences | Yes |
| `MEMORY.md` | General knowledge the agent should always have about this user | Yes |
| `memory/YYYY-MM-DD.md` | Daily conversation logs | Last 3 days |

### Daily Logs

The `memory/` directory inside each user's directory contains daily logs named `YYYY-MM-DD.md`. After each conversation, the agent appends a summary. The **last 3 daily logs** are included in the bootstrap context.

## Context Bootstrap

Context is assembled in two layers:

**Agent-level** (`MemoryStore::bootstrap_context`):
1. `SOUL.md` (up to 20K characters)

**Per-user** (`UserMemoryView::bootstrap_context`):
1. `SOUL.md` (from agent store, up to 20K characters)
2. `USER.md` (from user directory, up to 20K characters)
3. `MEMORY.md` (from user directory, up to 20K characters)
4. Last 3 daily logs (most recent first, from user directory)

This context is injected into the system prompt so the agent always has its identity, user knowledge, and recent history.

## Full-Text Search

All markdown files are indexed in SQLite FTS5 with chunking for efficient retrieval:

| Parameter | Value |
|-----------|-------|
| Chunk size | 1600 characters (~400 tokens) |
| Overlap | 320 characters (~80 tokens) |
| Splitting | Line-aware (never splits mid-line) |

Search results include the source file, matching text, line range, and relevance rank.

## Agent Tools

### MemorySearch

Search the full-text index:

```json
{
  "query": "user's favorite programming language",
  "limit": 5
}
```

### MemoryWrite

Write or update a file:

```json
{
  "file": "notes/rust-patterns.md",
  "content": "# Rust Patterns\n\n..."
}
```

### MemoryAppendDaily

Append to today's daily log:

```json
{
  "text": "User asked about database migrations"
}
```

## Background Review

Starpod can automatically review conversations and persist important information to memory (and skills) — without the agent needing to decide inline during the conversation.

Every **N user messages** (configurable via `memory.nudge_interval`, default: 10), a background LLM call:

1. Loads the full session transcript from the database
2. Reviews it for important information
3. Persists findings using `MemoryWrite` and `MemoryAppendDaily`:
   - User preferences and personal details → `USER.md`
   - Key decisions, facts, and technical context → `MEMORY.md`
   - Time-specific notes and conversation summaries → daily log
4. When **self-improve** is enabled, also creates or updates skills via `SkillCreate` / `SkillUpdate` for complex workflows discovered in the conversation

The nudge runs in a background task and never blocks the main chat flow. If the LLM call fails, a warning is logged and the conversation continues unaffected (fail-open).

### Short conversations

Conversations that don't reach the nudge interval (e.g., 3 messages with `nudge_interval = 10`) are handled by two mechanisms:

- **Session switch**: when a user sends a message to a different session, any un-nudged sessions they left behind are automatically flushed in the background. This is the primary safety net for the web UI, where sessions are long-lived and never close.
- **Session close**: when a gap-based session closes (e.g., Telegram after 6 hours of inactivity), a final nudge runs if there are un-nudged messages.

In both cases, the nudge reviews the full transcript and persists important information, so no conversation is lost.

### Configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `nudge_interval` | integer | `10` | User messages between nudges. Set to `0` to disable |
| `nudge_model` | string | — | Model override. Falls back to `compaction.flush_model` → `compaction_model` → primary model |
| `self_improve` | bool | `false` | Also create/update skills during background reviews (top-level setting) |

```toml
[memory]
nudge_interval = 10
nudge_model = "anthropic/claude-haiku-4-5-20251001"
```

::: tip
Use a fast, cheap model (like Haiku) for nudges — they don't need the full power of your primary model.
:::

### Background Review vs. Pre-Compaction Flush

Starpod has two mechanisms for background memory persistence:

| | Background Review | Flush |
|---|---|---|
| **When** | Every N user messages (+ session switch/close) | Before context compaction |
| **Trigger** | Message count / session switch | Context window filling up |
| **Scope** | Full session transcript | Messages being discarded |
| **Tools** | Memory + Skills (when self-improve on) | Memory only |
| **Config** | `memory.nudge_interval` | `compaction.memory_flush` |

Both can be active simultaneously. The background review catches information proactively; the flush is a safety net before context is lost.


## Manual Editing

You can edit any file in `.starpod/` with your text editor. The search index is rebuilt automatically on the next server start, or you can trigger a reindex via the API (`POST /api/memory/reindex`).

::: tip
Edit `SOUL.md` to change the agent's personality. Edit `USER.md` to update what the agent knows about you. Changes take effect on the next conversation.
:::
