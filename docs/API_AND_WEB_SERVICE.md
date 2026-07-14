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
EXPOSE 3000
ENTRYPOINT ["htmlmd-server"]
```

Build and run:

```bash
docker build -t htmlmd-server -f crates/htmlmd-server/Dockerfile .
docker run -p 3000:3000 htmlmd-server
```

## Notes

- The server currently listens on `127.0.0.1:3000`. For production, run it behind a reverse proxy and restrict access as needed.
- The API returns `422 Unprocessable Entity` if conversion fails (for example, because of a strict-mode limit error).
- For high-throughput deployments, run multiple instances behind a load balancer; the conversion itself is CPU-bound.
