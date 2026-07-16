# API and web service

`htmlmd` ships as a Rust library and a CLI, and includes a small HTTP server crate (`crates/htmlmd-server`) that exposes the conversion engine as a JSON API.

## Start the server

```bash
cargo run -p htmlmd-server --release
```

The server binds to `127.0.0.1:3000` by default.

To run the release binary directly:

```bash
cargo build -p htmlmd-server --release
./target/release/htmlmd-server
```

You should see:

```text
INFO htmlmd_server: htmlmd server listening on http://127.0.0.1:3000
```

To listen on a different address or port:

```bash
./target/release/htmlmd-server --bind 0.0.0.0:8080
```

The server shuts down gracefully on `Ctrl-C` (SIGINT) and, on Unix, SIGTERM: in-flight requests are allowed to finish before the process exits.

## Configuration

| Flag / variable | Description | Default |
| --- | --- | --- |
| `--bind <ADDR:PORT>` | Address and port to listen on | `127.0.0.1:3000` |
| `HTMLMD_BIND` | Bind address, used when `--bind` is absent | `127.0.0.1:3000` |
| `HTMLMD_MAX_BODY_BYTES` | Maximum request body size in bytes; larger bodies get `413 Payload Too Large` | `67108864` (64 MiB) |
| `-h`, `--help` | Print usage and exit | |
| `-V`, `--version` | Print version and exit | |

The `--bind` flag takes precedence over `HTMLMD_BIND`. An invalid bind address or `HTMLMD_MAX_BODY_BYTES` value prints an error to stderr and exits with code 2; a failure to bind the socket (for example, port already in use) exits with code 1.

## Endpoints

### `GET /health`

Simple health check.

```bash
curl http://127.0.0.1:3000/health
```

Response:

```text
ok
```

### `POST /convert`

Convert an HTML document to Markdown.

#### Request body

```json
{
  "html": "<h1>Hello</h1><p>This is <b>bold</b>.</p>",
  "options": {
    "profile": "obsidian",
    "cleanup": {
      "metadata": {
        "title": true,
        "description": true,
        "canonical-url": true
      },
      "image-mode": "reference"
    },
    "render": {
      "link-style": "reference",
      "reference-placement": "adjacent"
    }
  }
}
```

- `html` (string, required): the HTML source.
- `options` (object, optional): any `ConversionOptions` fields. If omitted, default options are used.

All option names use the same kebab-case spelling as the config file, for example `reference-placement`, `image-mode`, `definition-lists`.

#### Response

```json
{
  "markdown": "# Hello\n\nThis is **bold**.",
  "title": null,
  "description": null,
  "canonical_url": null,
  "diagnostics": []
}
```

## Example requests

### Basic conversion

```bash
curl -s -X POST http://127.0.0.1:3000/convert \
  -H 'Content-Type: application/json' \
  -d '{"html": "<h1>Hello</h1>"}'
```

### Obsidian profile with frontmatter

```bash
curl -s -X POST http://127.0.0.1:3000/convert \
  -H 'Content-Type: application/json' \
  -d '{
    "html": "<title>My page</title><p>This is <mark>important</mark>.</p>",
    "options": {
      "profile": "obsidian",
      "cleanup": {
        "metadata": { "title": true }
      }
    }
  }'
```

Response:

```json
{
  "markdown": "---\ntitle: My page\n---\nThis is ==important==.",
  "title": "My page",
  "description": null,
  "canonical_url": null,
  "diagnostics": []
}
```

### GFM tables

```bash
curl -s -X POST http://127.0.0.1:3000/convert \
  -H 'Content-Type: application/json' \
  -d '{
    "html": "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>",
    "options": { "profile": "gfm" }
  }'
```

## Deploying the service

### Using systemd

Create `/etc/systemd/system/htmlmd-server.service`:

```ini
[Unit]
Description=htmlmd conversion API
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/htmlmd-server
Environment=HTMLMD_BIND=127.0.0.1:3000
Restart=on-failure
User=www-data
Group=www-data

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now htmlmd-server
```

### Behind nginx

```nginx
server {
    listen 80;
    server_name htmlmd.example.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### Docker

```dockerfile
FROM rust:1.94 AS builder
WORKDIR /app
COPY . .
RUN cargo build -p htmlmd-server --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/htmlmd-server /usr/local/bin/htmlmd-server
ENV HTMLMD_BIND=0.0.0.0:3000
EXPOSE 3000
ENTRYPOINT ["htmlmd-server"]
```

Build and run:

```bash
docker build -t htmlmd-server -f crates/htmlmd-server/Dockerfile .
docker run -p 3000:3000 htmlmd-server
```

## Notes

- The server listens on `127.0.0.1:3000` by default; use `--bind` or `HTMLMD_BIND` to change that. For production, run it behind a reverse proxy and restrict access as needed.
- The API returns `422 Unprocessable Entity` if conversion fails (for example, because of a strict-mode limit error), `400 Bad Request` for malformed JSON, and `413 Payload Too Large` when the request body exceeds `HTMLMD_MAX_BODY_BYTES`.
- For high-throughput deployments, run multiple instances behind a load balancer; the conversion itself is CPU-bound.
