# Production server deployment

This guide runs `htmlmd-server` as a private backend behind Apache HTTP Server
2.4 or Nginx. For request and response details, see
[API and web service](API_AND_WEB_SERVICE.md).

## Recommended topology

```text
client ──HTTPS──> Apache or Nginx :443 ──HTTP──> 127.0.0.1:3000
                                                  htmlmd-server
```

Keep the backend bound to `127.0.0.1`, terminate TLS at the reverse proxy, and
apply authentication, request-size limits, and timeouts there. The default
`htmlmd-server` bind already follows this model:

```bash
HTMLMD_BIND=127.0.0.1:3000 \
HTMLMD_MAX_BODY_BYTES=8388608 \
  /usr/local/bin/htmlmd-server
```

Check the private endpoint before configuring a proxy:

```bash
curl --fail http://127.0.0.1:3000/health
ss -ltnp | grep ':3000'
```

The listener should show `127.0.0.1:3000`, not `0.0.0.0:3000`. If the proxy
and backend run in different containers or hosts, use a private network instead
of loopback and do not publish the backend port publicly.

The examples below use an 8 MiB request limit. Keep the proxy limit at or below
`HTMLMD_MAX_BODY_BYTES`, and adjust both values together if legitimate pages
are larger.

## Keep the process running with systemd

`htmlmd-server` handles SIGINT and SIGTERM gracefully, but it does not daemonize
or restart itself. Use the operating system's service manager rather than a
custom watcher script.

Create a dedicated service account once:

```bash
sudo useradd --system --home /nonexistent --shell /usr/sbin/nologin htmlmd
```

Install the binary at `/usr/local/bin/htmlmd-server`, then create
`/etc/systemd/system/htmlmd-server.service`:

```ini
[Unit]
Description=htmlmd conversion API
After=network.target
StartLimitIntervalSec=60
StartLimitBurst=5

[Service]
Type=simple
User=htmlmd
Group=htmlmd
ExecStart=/usr/local/bin/htmlmd-server --bind 127.0.0.1:3000
Environment=HTMLMD_MAX_BODY_BYTES=8388608
Restart=on-failure
RestartSec=2s
TimeoutStopSec=30s

# The server needs no filesystem writes or Linux capabilities.
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictAddressFamilies=AF_INET AF_INET6
LockPersonality=true
CapabilityBoundingSet=

[Install]
WantedBy=multi-user.target
```

Load, enable, and inspect the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now htmlmd-server
sudo systemctl status htmlmd-server
sudo journalctl -u htmlmd-server -f
```

`Restart=on-failure` restarts the process after a crash, non-zero exit, or
abnormal signal. A deliberate `systemctl stop` does not trigger a restart.
`RestartSec=2s` avoids a tight crash loop, while the start-limit settings stop
permanent failures from retrying forever.

The `/health` endpoint proves that the process can answer an HTTP request; it is
not a full conversion self-test. systemd restarts exited processes but does not
poll this endpoint. If recovery from a hung process is required, monitor
`/health` externally or use an orchestrator with active health checks.

## Nginx reverse proxy

Create `/etc/nginx/sites-available/htmlmd` (paths may differ by distribution):

```nginx
server {
    listen 80;
    server_name htmlmd.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl;
    server_name htmlmd.example.com;

    ssl_certificate     /etc/letsencrypt/live/htmlmd.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/htmlmd.example.com/privkey.pem;

    client_max_body_size 8m;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;

        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Connection        "";

        proxy_connect_timeout 3s;
        proxy_send_timeout 35s;
        proxy_read_timeout 35s;
    }
}
```

Enable and validate it:

```bash
sudo ln -s /etc/nginx/sites-available/htmlmd /etc/nginx/sites-enabled/htmlmd
sudo nginx -t
sudo systemctl reload nginx
curl --fail https://htmlmd.example.com/health
```

`proxy_pass` has no URI suffix here, so `/health` and `/convert` reach the
backend unchanged. The proxy timeouts bound how long clients wait; they do not
kill CPU work that the backend has already started.

## Apache HTTP Server 2.4 reverse proxy

On Debian or Ubuntu, enable the required modules:

```bash
sudo a2enmod proxy proxy_http ssl headers
```

Create `/etc/apache2/sites-available/htmlmd.conf`:

```apache
<VirtualHost *:80>
    ServerName htmlmd.example.com
    Redirect permanent / https://htmlmd.example.com/
</VirtualHost>

<VirtualHost *:443>
    ServerName htmlmd.example.com

    SSLEngine on
    SSLCertificateFile /etc/letsencrypt/live/htmlmd.example.com/fullchain.pem
    SSLCertificateKeyFile /etc/letsencrypt/live/htmlmd.example.com/privkey.pem

    ProxyRequests Off
    ProxyPreserveHost On
    LimitRequestBody 8388608

    RequestHeader set X-Forwarded-Proto "https"
    RequestHeader set X-Forwarded-Port "443"

    ProxyPass        "/" "http://127.0.0.1:3000/" connectiontimeout=3 timeout=35
    ProxyPassReverse "/" "http://127.0.0.1:3000/"

    ErrorLog ${APACHE_LOG_DIR}/htmlmd-error.log
    CustomLog ${APACHE_LOG_DIR}/htmlmd-access.log combined
</VirtualHost>
```

Enable and validate it:

```bash
sudo a2ensite htmlmd
sudo apachectl configtest
sudo systemctl reload apache2
curl --fail https://htmlmd.example.com/health
```

`ProxyRequests Off` keeps forward-proxy behavior disabled. `ProxyPass` sends
requests to the private service, and `ProxyPassReverse` rewrites backend
redirect headers if any are introduced later.

## Authentication

`htmlmd-server` currently has no native authentication. Protect the public
virtual host at the reverse proxy and use HTTPS for every authenticated
request. Choose one authentication method; do not stack the examples below
unless that is intentional.

### Basic Auth: simplest small deployment

Create a bcrypt password file. Do not use `-b`, which exposes the password in
the process list and shell history:

```bash
sudo htpasswd -cB /etc/htmlmd.htpasswd htmlmd-api
sudo chown root:www-data /etc/htmlmd.htpasswd
sudo chmod 640 /etc/htmlmd.htpasswd
```

Use the group that runs your proxy instead of `www-data` when the distribution
uses another account. When adding later users, omit `-c` so the existing file
is not replaced.

For Nginx, add these directives inside the existing `location /` block:

```nginx
auth_basic "htmlmd API";
auth_basic_user_file /etc/htmlmd.htpasswd;
```

For Apache, enable the auth modules and add the `Location` block inside the
TLS virtual host:

```bash
sudo a2enmod auth_basic authn_file authz_user
```

```apache
<Location "/">
    AuthType Basic
    AuthName "htmlmd API"
    AuthBasicProvider file
    AuthUserFile /etc/htmlmd.htpasswd
    Require valid-user
</Location>
```

Test either proxy with:

```bash
curl --fail --user htmlmd-api https://htmlmd.example.com/health
```

### Static bearer-header gate

For a small service with a few trusted callers, the proxy can require one
random static token. This is an exact header check, not OAuth token validation:
there are no scopes, expiry times, issuers, or revocation records. Generate a
64-character lowercase hexadecimal token and store it in a root-readable proxy
configuration file:

```bash
openssl rand -hex 32
```

For Nginx, put this `map` in the `http` context (for example,
`/etc/nginx/conf.d/htmlmd-bearer.conf`). Replace the placeholder with the exact
generated token:

```nginx
map $http_authorization $htmlmd_bearer_ok {
    default 0;
    ~^Bearer\x20REPLACE_WITH_64_LOWERCASE_HEX_CHARACTERS$ 1;
}
```

Then add this near the top of the existing `location /` block. This use of
`if` only returns a response; it does not rewrite request routing:

```nginx
if ($htmlmd_bearer_ok = 0) {
    add_header WWW-Authenticate 'Bearer realm="htmlmd"' always;
    return 401;
}

# Do not pass the credential to htmlmd-server after the proxy accepts it.
proxy_set_header Authorization "";
```

For Apache, enable `setenvif` and `authz_core`, then add the following inside
the TLS virtual host. Replace the placeholder with the exact token:

```bash
sudo a2enmod setenvif authz_core headers
```

```apache
SetEnvIf Authorization "^Bearer REPLACE_WITH_64_LOWERCASE_HEX_CHARACTERS$" htmlmd_bearer_ok

<Location "/">
    Require env htmlmd_bearer_ok
</Location>

# Do not pass the credential to htmlmd-server after the proxy accepts it.
RequestHeader unset Authorization
```

Apache's simple `Require env` gate returns `403` for a missing or incorrect
token; Nginx returns `401` in the configuration above. Test with:

```bash
curl --fail \
  -H 'Authorization: Bearer REPLACE_WITH_64_LOWERCASE_HEX_CHARACTERS' \
  https://htmlmd.example.com/health
```

Keep the token file out of source control, restrict its permissions, use TLS,
and rotate the token periodically. For multiple users, short-lived tokens,
revocation, or SSO, put an OAuth2/OIDC-aware gateway in front instead. Common
integration points are Nginx `auth_request`, Apache `mod_auth_openidc`, or a
managed API gateway.

## Containers and restart policies

The published container listens on `0.0.0.0:3000` inside the container. Bind
the published host port to loopback so it remains private, and let Docker
restart the container after process or host failure:

```bash
docker run -d \
  --name htmlmd-server \
  --restart unless-stopped \
  -p 127.0.0.1:3000:3000 \
  -e HTMLMD_MAX_BODY_BYTES=8388608 \
  ghcr.io/steamvogue/htmlmd-server:latest
```

A Docker restart policy reacts to container exit; it does not restart a
container merely because an application health check becomes unhealthy. Use an
orchestrator or external monitor when active health-based replacement is a
requirement.

## Production checklist

- Backend listens only on loopback or a private network.
- Public traffic uses HTTPS.
- Exactly one authentication method is enabled and tested.
- Proxy and backend body limits agree.
- Proxy connect/read/send timeouts are set.
- systemd, Docker, or an orchestrator owns process restart behavior.
- Local and proxied `/health` requests succeed.
- A real authenticated `POST /convert` request succeeds.
- Logs do not contain passwords or bearer tokens.

## Primary references

- [Apache HTTP Server 2.4 reverse proxy guide](https://httpd.apache.org/docs/2.4/howto/reverse_proxy.html)
- [Apache HTTP Server authentication and authorization](https://httpd.apache.org/docs/2.4/howto/auth.html)
- [Apache `ProxyPass` reference](https://httpd.apache.org/docs/2.4/mod/mod_proxy.html#proxypass)
- [Nginx proxy module](https://nginx.org/en/docs/http/ngx_http_proxy_module.html)
- [Nginx Basic Auth module](https://nginx.org/en/docs/http/ngx_http_auth_basic_module.html)
- [Nginx `map` module](https://nginx.org/en/docs/http/ngx_http_map_module.html)
- [systemd service restart behavior](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html#Restart=)
- [Docker restart policies](https://docs.docker.com/engine/containers/start-containers-automatically/)
