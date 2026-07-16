# htmlmd-server container image.
#
# Build context must be the WORKSPACE ROOT (the Cargo.lock / workspace
# Cargo.toml live there), not this directory:
#
#   docker build -f crates/htmlmd-server/Dockerfile -t htmlmd-server .
#
# .github/workflows/release.yml builds this as a multi-arch image
# (linux/amd64, linux/arm64) and pushes it to ghcr.io/steamvogue/htmlmd-server.
#
# Builder and runtime are pinned to the same Debian release (bookworm) so the
# glibc the binary links against is the glibc it runs on.
FROM rust:1.94-bookworm AS builder
WORKDIR /app
COPY . .
# mimalloc (default feature) needs a C compiler; the rust image ships gcc.
RUN cargo build -p htmlmd-server --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/htmlmd-server /usr/local/bin/htmlmd-server
# htmlmd-server binds 127.0.0.1:3000 by default; inside a container it must
# listen on all interfaces to be reachable. Override with -e HTMLMD_BIND=...
ENV HTMLMD_BIND=0.0.0.0:3000
EXPOSE 3000
ENTRYPOINT ["htmlmd-server"]
