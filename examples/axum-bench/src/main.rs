use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

// ─── App state (replaces webr config DI) ────────────────────────────

const APP_NAME: &str = "hello-world";
const APP_VERSION: &str = "1.0.0";
const APP_GREETING: &str = "Hello from WebR!";

// ─── Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub name: String,
}

// ─── Handlers ───────────────────────────────────────────────────────

async fn index() -> &'static str {
    APP_GREETING
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn info() -> String {
    format!("{} v{}", APP_NAME, APP_VERSION)
}

async fn trigger_panic() -> &'static str {
    panic!("Something went terribly wrong!");
}

async fn list_items() -> Json<Vec<Item>> {
    Json(vec![
        Item {
            id: 1,
            name: "Alice".into(),
        },
        Item {
            id: 2,
            name: "Bob".into(),
        },
    ])
}

async fn get_item(Path(id): Path<i64>) -> Result<Json<Item>, StatusCode> {
    if id > 0 && id <= 2 {
        Ok(Json(Item {
            id,
            name: format!("Item-{id}"),
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn create_item(Json(body): Json<CreateItemRequest>) -> Json<Item> {
    Json(Item {
        id: 42,
        name: body.name,
    })
}

async fn update_item(
    Path(id): Path<i64>,
    Json(body): Json<CreateItemRequest>,
) -> Json<Item> {
    Json(Item {
        id,
        name: body.name,
    })
}

async fn delete_item(Path(id): Path<i64>) -> StatusCode {
    let _ = id;
    StatusCode::NO_CONTENT
}

// ─── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new("info").unwrap())
        .init();

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/info", get(info))
        .route("/panic", get(trigger_panic))
        .route("/items", get(list_items).post(create_item))
        .route("/items/{id}", get(get_item).put(update_item).delete(delete_item))
        .layer(axum::middleware::from_fn(panic_recovery));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// Panic recovery middleware
async fn panic_recovery(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    use std::panic::AssertUnwindSafe;
    use futures_util::FutureExt;

    let result = AssertUnwindSafe(next.run(req)).catch_unwind().await;
    match result {
        Ok(response) => response,
        Err(err) => {
            tracing::error!("Request handler panicked: {:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown...");
}
