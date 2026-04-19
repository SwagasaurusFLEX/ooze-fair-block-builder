use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

use ooze_fair_block_builder::token_forensics::{analyze_token, ForensicsReport};

#[derive(Clone)]
struct AppState {
    api_key: String,
    static_dir: String,
}

#[derive(Deserialize)]
struct AnalyzeRequest {
    mint: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[tokio::main]
async fn main() {
    let api_key = env::var("SOLTRACKER_API_KEY")
        .expect("SOLTRACKER_API_KEY environment variable must be set");

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "web".to_string());

    let state = Arc::new(AppState {
        api_key,
        static_dir: static_dir.clone(),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/analyze", post(analyze_post))
        .route("/api/analyze/:mint", get(analyze_get))
        .route("/analyze", get(serve_analyze))
        .fallback_service(ServeDir::new(&static_dir))
        .with_state(state)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("OOZE web server listening on http://localhost:3000");
    println!("   Bind address: http://{}", addr);
    println!("   Static files:  {}", static_dir);
    println!("   API endpoint:  POST http://{}/api/analyze", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn serve_analyze(State(state): State<Arc<AppState>>) -> Response {
    let path = std::path::Path::new(&state.static_dir).join("analyze.html");
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => (
            StatusCode::OK,
            [("content-type", "text/html; charset=utf-8")],
            contents,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "analyze.html not found").into_response(),
    }
}

async fn analyze_post(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AnalyzeRequest>,
) -> Response {
    run_analysis(state.api_key.clone(), req.mint).await
}

async fn analyze_get(State(state): State<Arc<AppState>>, Path(mint): Path<String>) -> Response {
    run_analysis(state.api_key.clone(), mint).await
}

async fn run_analysis(api_key: String, mint: String) -> Response {
    // Validation
    if mint.len() < 32 || mint.len() > 64 {
        return error_response(StatusCode::BAD_REQUEST, "Invalid mint address length");
    }
    if !mint.chars().all(|c| c.is_ascii_alphanumeric()) {
        return error_response(StatusCode::BAD_REQUEST, "Invalid mint address characters");
    }

    println!("  Analyzing {}", mint);

    // Convert the !Send error to a String immediately so nothing survives across awaits
    let result: Result<ForensicsReport, String> = match analyze_token(&api_key, &mint).await {
        Ok(r) => Ok(r),
        Err(e) => Err(e.to_string()),
    };

    match result {
        Ok(report) => Json(report).into_response(),
        Err(msg) => {
            eprintln!("  Failed: {}", msg);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Analysis failed: {}", msg),
            )
        }
    }
}

fn error_response(code: StatusCode, msg: &str) -> Response {
    (
        code,
        Json(ErrorBody {
            error: msg.to_string(),
        }),
    )
        .into_response()
}