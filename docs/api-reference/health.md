# Health

## GET /api/health

Simple health check endpoint. No authentication required.

```bash
curl http://localhost:3000/api/health
```

### Response

```json
{
  "status": "ok"
}
```

Returns `200 OK` when the server is running.
