# CLI Reference

The `orion` binary provides commands for all major features.

## Agent

### `orion agent init`

Initialize a new `.orion/` project directory.

```bash
orion agent init                  # Interactive wizard
orion agent init --default        # Skip wizard, use defaults
orion agent init --name "Alice" --model "claude-opus-4-6"
```

| Flag | Description |
|------|-------------|
| `--name` | Your display name |
| `--timezone` | IANA timezone |
| `--agent-name` | Agent's display name |
| `--soul` | Personality/instructions |
| `--model` | Claude model |
| `--default` | Skip the wizard |

### `orion agent serve`

Start the HTTP/WS server with optional Telegram bot.

```bash
orion agent serve
```

Serves the web UI, REST API, WebSocket endpoint, and (if configured) Telegram bot. All share the same agent instance.

### `orion agent chat`

Send a one-shot message.

```bash
orion agent chat "What files are in this directory?"
```

### `orion agent repl`

Start an interactive REPL with readline support and history.

```bash
orion agent repl
```

## Memory

### `orion agent memory search`

Full-text search across memory and knowledge files.

```bash
orion agent memory search "database migrations"
orion agent memory search "rust patterns" --limit 10
```

| Flag | Default | Description |
|------|---------|-------------|
| `--limit`, `-l` | `5` | Maximum results |

### `orion agent memory reindex`

Rebuild the FTS5 search index.

```bash
orion agent memory reindex
```

## Vault

### `orion agent vault set`

Encrypt and store a credential.

```bash
orion agent vault set github_token "ghp_xxxxxxxxxxxx"
```

### `orion agent vault get`

Retrieve a decrypted credential.

```bash
orion agent vault get github_token
```

### `orion agent vault delete`

Delete a stored credential.

```bash
orion agent vault delete github_token
```

### `orion agent vault list`

List all stored keys (values are not shown).

```bash
orion agent vault list
```

## Sessions

### `orion agent sessions list`

List recent sessions.

```bash
orion agent sessions list
orion agent sessions list --limit 20
```

| Flag | Default | Description |
|------|---------|-------------|
| `--limit`, `-l` | `10` | Maximum sessions |

## Skills

### `orion agent skills list`

List all skills.

```bash
orion agent skills list
```

### `orion agent skills show`

Show a skill's content.

```bash
orion agent skills show code-review
```

### `orion agent skills create`

Create a new skill.

```bash
orion agent skills create "code-review" --content "Always check for error handling"
orion agent skills create "code-review" --file code-review.md
```

| Flag | Description |
|------|-------------|
| `--content`, `-c` | Inline skill content |
| `--file`, `-f` | Read content from a file |

### `orion agent skills delete`

Delete a skill.

```bash
orion agent skills delete code-review
```

## Cron

### `orion agent cron list`

List all scheduled jobs.

```bash
orion agent cron list
```

### `orion agent cron remove`

Remove a job by name.

```bash
orion agent cron remove "morning-reminder"
```

### `orion agent cron runs`

Show recent executions for a job.

```bash
orion agent cron runs "morning-reminder"
orion agent cron runs "morning-reminder" --limit 20
```

| Flag | Default | Description |
|------|---------|-------------|
| `--limit`, `-l` | `10` | Maximum runs |

## Instances

Manage remote cloud instances. Requires `instance_backend_url` in config or `ORION_INSTANCE_BACKEND_URL` env var.

### `orion instance create`

Create a new remote instance.

```bash
orion instance create
orion instance create --name "my-bot" --region "us-east-1"
```

| Flag | Description |
|------|-------------|
| `--name`, `-n` | Display name for the instance |
| `--region`, `-r` | Deployment region |

### `orion instance list`

List all instances with status and region.

```bash
orion instance list
```

### `orion instance kill`

Terminate a running instance.

```bash
orion instance kill <id>
```

### `orion instance pause`

Suspend a running instance.

```bash
orion instance pause <id>
```

### `orion instance restart`

Resume a paused instance.

```bash
orion instance restart <id>
```

### `orion instance logs`

Stream logs from a running instance. Output is colored by log level (error=red, warn=yellow, info=green, debug=dim).

```bash
orion instance logs <id>
orion instance logs <id> --tail 100
```

| Flag | Default | Description |
|------|---------|-------------|
| `--tail`, `-t` | `50` | Number of recent log lines to stream |

### `orion instance ssh`

Open an SSH session to a running instance. Fetches connection info from the backend and spawns a native `ssh` process. Ephemeral keys are written to a temp file and cleaned up after the session.

```bash
orion instance ssh <id>
```

### `orion instance health`

Display health metrics for an instance: CPU%, memory, disk, uptime, and last heartbeat.

```bash
orion instance health <id>
```
