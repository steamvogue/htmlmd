use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use htmlmd_core::{
    convert, ConversionOptions, ConversionResult,
    diagnostic::Diagnostic,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/convert", post(convert_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("htmlmd server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
