# Instances

Orion can manage **remote cloud instances** through a backend API. You can create, list, pause, restart, and kill instances — plus stream logs, open SSH sessions, and monitor health with automatic restart on stale heartbeats.

## Configuration

Add the backend URL to your `.orion/config.toml`:

```toml
instance_backend_url = "https://api.orion.example.com"
```

Or set the environment variable:

```bash
export ORION_INSTANCE_BACKEND_URL="https://api.orion.example.com"
```

The API key for authentication is resolved from `providers.anthropic.api_key` or the `ANTHROPIC_API_KEY` environment variable.

## Instance Lifecycle

Each instance has a status that tracks its current state:

| Status | Description |
|--------|-------------|
| `Creating` | Instance is being provisioned |
| `Running` | Instance is active and healthy |
| `Paused` | Instance is suspended (can be restarted) |
| `Stopped` | Instance has been terminated |
| `Error` | Instance encountered a failure |

```
Creating → Running → Paused → Running (restart)
                   → Stopped (kill)
                   → Error
```

## Log Streaming

Logs are streamed as newline-delimited JSON from the backend. Each log entry contains a timestamp, level, and message. The CLI displays logs with colored output:

- **Error** — red
- **Warn** — yellow
- **Info** — green
- **Debug** — dim

```bash
orion instance logs <id>            # Stream last 50 lines
orion instance logs <id> --tail 100 # Stream last 100 lines
```

## SSH Access

Orion fetches SSH connection info from the backend and spawns a native `ssh` process. If the backend provides an ephemeral private key, it is written to a temporary file with `0600` permissions and cleaned up after the session ends.

```bash
orion instance ssh <id>
```

## Health Monitoring

The `HealthMonitor` polls instance health at a configurable interval (default: 30 seconds). If the heartbeat becomes stale (default timeout: 90 seconds), it automatically triggers a restart.

Health data includes:

| Metric | Description |
|--------|-------------|
| `cpu_percent` | CPU utilization percentage |
| `memory_mb` | Memory usage in MB |
| `disk_mb` | Disk usage in MB |
| `last_heartbeat` | Timestamp of last heartbeat |
| `uptime_secs` | Seconds since instance started |

### Status Change Callbacks

You can register callbacks that fire when an instance's status changes — useful for alerting or logging:

```rust
use orion_instances::{InstanceClient, HealthMonitor};

let client = InstanceClient::new("https://api.example.com", Some("api-key"));
let monitor = HealthMonitor::new(client, "instance-id")
    .with_interval(Duration::from_secs(30))
    .with_heartbeat_timeout(Duration::from_secs(90))
    .on_status_change(|id, old, new| {
        println!("Instance {id} changed from {old:?} to {new:?}");
    });

let shutdown = monitor.start().await;
// ... later ...
let _ = shutdown.send(());  // Stop monitoring
```

## CLI

```bash
orion instance create                         # Create a new instance
orion instance create --name "my-bot" --region "us-east-1"
orion instance list                           # List all instances
orion instance kill <id>                      # Terminate an instance
orion instance pause <id>                     # Suspend an instance
orion instance restart <id>                   # Resume a paused instance
orion instance logs <id> [--tail N]           # Stream logs (default: 50 lines)
orion instance ssh <id>                       # SSH into an instance
orion instance health <id>                    # Check instance health
```

## Gateway API

The gateway exposes instance management over HTTP. See the [API Reference](/api-reference/instances) for full details.
