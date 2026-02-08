# Corto

A minimal URL shortener built with Axum + SeaORM (PostgreSQL).

## Features
- RESTful API
- RFC 9457 problem+json errors
- Base62 short code (custom alphabet)
- Admin endpoints

## Requirements
- Rust stable
- PostgreSQL

## Configuration
Edit `config.toml`:
```toml
[server]
port = 3000
base_url = "http://localhost:3000"

[datasource]
url = "postgres://user:pass@127.0.0.1:5432/corto"

[logging]
level = "info"
```

## Run
```bash
cargo run
```

## API
### Create short url
```bash
curl -X POST http://localhost:3000/api/short-urls \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://example.com","expiresAt":"2026-02-28T12:00:00Z"}'
```

### Resolve short url
```bash
curl http://localhost:3000/api/short-urls/{code}
```

### Redirect
```bash
curl -I http://localhost:3000/{code}
```

### Admin list
```bash
curl "http://localhost:3000/admin/short-urls?page=1&pageSize=20"
```

### Admin update
```bash
curl -X PATCH http://localhost:3000/admin/short-urls/{id} \
  -H 'Content-Type: application/json' \
  -d '{"status":0,"expiresAt":""}'
```

### Admin delete
```bash
curl -X DELETE http://localhost:3000/admin/short-urls/{id}
```
