// SPDX-License-Identifier: MIT OR Apache-2.0

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
};
use htmlmd_core::{ConversionOptions, ConversionResult, convert, diagnostic::Diagnostic};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::process::ExitCode;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const DEFAULT_BIND: &str = "127.0.0.1:3000";
const DEFAULT_MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

const USAGE: &str = "\
htmlmd-server - HTTP server exposing the htmlmd conversion API

Usage: htmlmd-server [OPTIONS]

Options:
      --bind <ADDR:PORT>  Address to listen on [env: HTMLMD_BIND] [default: 127.0.0.1:3000]
  -h, --help              Print help
  -V, --version           Print version

Environment:
  HTMLMD_BIND            Bind address, used when --bind is absent
  HTMLMD_MAX_BODY_BYTES  Maximum request body size in bytes [default: 67108864 (64 MiB)]
";

#[derive(Deserialize)]
struct ConvertRequest {
    html: String,
    #[serde(default)]
    options: Option<ConversionOptions>,
}

#[derive(Serialize)]
struct ConvertResponse {
    markdown: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    canonical_url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    diagnostics: Vec<Diagnostic>,
}

impl From<ConversionResult> for ConvertResponse {
    fn from(result: ConversionResult) -> Self {
        Self {
            markdown: result.markdown,
            title: result.title,
            description: result.description,
            canonical_url: result.canonical_url,
            diagnostics: result.diagnostics,
        }
    }
}

async fn convert_handler(
    Json(req): Json<ConvertRequest>,
) -> Result<Json<ConvertResponse>, StatusCode> {
    let options = req.options.unwrap_or_default();
    match convert(&req.html, &options) {
        Ok(result) => Ok(Json(result.into())),
        Err(err) => {
            tracing::error!("conversion failed: {err}");
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        }
    }
}

async fn health_handler() -> &'static str {
    "ok"
}

pub(crate) fn app() -> Router {
    app_with_body_limit(DEFAULT_MAX_BODY_BYTES)
}

fn app_with_body_limit(max_body_bytes: usize) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/convert", post(convert_handler))
        .layer(DefaultBodyLimit::max(max_body_bytes))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!("failed to install ctrl-c handler: {err}");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(err) => {
                tracing::error!("failed to install SIGTERM handler: {err}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received, stopping server");
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut bind_flag: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("htmlmd-server {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--bind" => match args.next() {
                Some(value) => bind_flag = Some(value),
                None => {
                    eprintln!("error: --bind requires an <addr:port> argument");
                    return ExitCode::from(2);
                }
            },
            other => {
                if let Some(value) = other.strip_prefix("--bind=") {
                    bind_flag = Some(value.to_string());
                } else {
                    eprintln!("error: unrecognized argument '{other}'");
                    eprint!("{USAGE}");
                    return ExitCode::from(2);
                }
            }
        }
    }

    let bind_str = bind_flag
        .or_else(|| std::env::var("HTMLMD_BIND").ok())
        .unwrap_or_else(|| DEFAULT_BIND.to_string());
    let addr: SocketAddr = match bind_str.parse() {
        Ok(addr) => addr,
        Err(err) => {
            eprintln!("error: invalid bind address '{bind_str}': {err}");
            return ExitCode::from(2);
        }
    };

    let router = match std::env::var("HTMLMD_MAX_BODY_BYTES") {
        Ok(raw) => match raw.parse::<usize>() {
            Ok(max_body_bytes) => app_with_body_limit(max_body_bytes),
            Err(err) => {
                eprintln!("error: invalid HTMLMD_MAX_BODY_BYTES '{raw}': {err}");
                return ExitCode::from(2);
            }
        },
        Err(_) => app(),
    };

    tracing_subscriber::fmt::init();

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("failed to bind {addr}: {err}");
            return ExitCode::FAILURE;
        }
    };
    tracing::info!("htmlmd server listening on http://{addr}");

    if let Err(err) = axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("server error: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn post_json(uri: &str, payload: &serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    }

    async fn body_string(response: axum::response::Response) -> String {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_string(response).await, "ok");
    }

    #[tokio::test]
    async fn convert_returns_markdown() {
        let payload = serde_json::json!({ "html": "<h1>Hi</h1>" });
        let response = app()
            .oneshot(post_json("/convert", &payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_str(&body_string(response).await).unwrap();
        assert!(body["markdown"].as_str().unwrap().contains("# Hi"));
    }

    #[tokio::test]
    async fn convert_rejects_invalid_json() {
        let request = Request::builder()
            .method("POST")
            .uri("/convert")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{not json"))
            .unwrap();
        let response = app().oneshot(request).await.unwrap();
        assert!(response.status().is_client_error());
    }

    #[tokio::test]
    async fn convert_strict_limit_returns_422() {
        let payload = serde_json::json!({
            "html": "<h1>Hi</h1>",
            "options": { "strict": true, "limits": { "max-input-bytes": 1 } }
        });
        let response = app()
            .oneshot(post_json("/convert", &payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
