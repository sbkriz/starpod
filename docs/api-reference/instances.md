# Instances

## List Instances {#list-instances}

### GET /api/instances

```bash
curl http://localhost:3000/api/instances \
  -H "X-API-Key: your-key"
```

#### Response

```json
[
  {
    "id": "inst_abc123",
    "name": "my-bot",
    "status": "Running",
    "region": "us-east-1",
    "created_at": "2026-03-14T10:00:00Z",
    "updated_at": "2026-03-14T10:05:00Z",
    "health": {
      "cpu_percent": 12.5,
      "memory_mb": 256,
      "disk_mb": 1024,
      "last_heartbeat": "2026-03-14T10:05:00Z",
      "uptime_secs": 3600
    }
  }
]
```

## Create Instance {#create-instance}

### POST /api/instances

```bash
curl -X POST http://localhost:3000/api/instances \
  -H "X-API-Key: your-key" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-bot", "region": "us-east-1"}'
```

#### Request Body

| Field | Type | Required | Description |
|-------|------|:--------:|-------------|
| `name` | string | No | Display name for the instance |
| `region` | string | No | Deployment region |

#### Response

Returns `201 Created` with the created instance object.

## Get Instance {#get-instance}

### GET /api/instances/:id

```bash
curl http://localhost:3000/api/instances/inst_abc123 \
  -H "X-API-Key: your-key"
```

#### Response

Returns a single instance object (same shape as list items), or `404` if not found.

## Delete Instance {#delete-instance}

### DELETE /api/instances/:id

```bash
curl -X DELETE http://localhost:3000/api/instances/inst_abc123 \
  -H "X-API-Key: your-key"
```

#### Response

Returns `204 No Content` on success.

## Pause Instance {#pause-instance}

### POST /api/instances/:id/pause

```bash
curl -X POST http://localhost:3000/api/instances/inst_abc123/pause \
  -H "X-API-Key: your-key"
```

#### Response

Returns the updated instance object with `status: "Paused"`.

## Restart Instance {#restart-instance}

### POST /api/instances/:id/restart

```bash
curl -X POST http://localhost:3000/api/instances/inst_abc123/restart \
  -H "X-API-Key: your-key"
```

#### Response

Returns the updated instance object with `status: "Running"`.

## Instance Health {#instance-health}

### GET /api/instances/:id/health

```bash
curl http://localhost:3000/api/instances/inst_abc123/health \
  -H "X-API-Key: your-key"
```

#### Response

```json
{
  "cpu_percent": 12.5,
  "memory_mb": 256,
  "disk_mb": 1024,
  "last_heartbeat": "2026-03-14T10:05:00Z",
  "uptime_secs": 3600
}
```

## Instance Statuses

| Status | Description |
|--------|-------------|
| `Creating` | Provisioning in progress |
| `Running` | Active and healthy |
| `Paused` | Suspended, can be restarted |
| `Stopped` | Terminated |
| `Error` | Encountered a failure |

::: info
Log streaming (`GET /instances/:id/logs`) and SSH info (`GET /instances/:id/ssh`) are available via the CLI only. See the [CLI Reference](/cli-reference#instances).
:::
