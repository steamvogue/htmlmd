FROM rust:1.94 AS builder
WORKDIR /app
COPY . .
RUN cargo build -p htmlmd-server --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/htmlmd-server /usr/local/bin/htmlmd-server
EXPOSE 3000
ENTRYPOINT ["htmlmd-server"]
