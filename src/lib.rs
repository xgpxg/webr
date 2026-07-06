pub use webr_web::app::AppBuilder;
pub use webr_web::component::{
    Component, ComponentEntry, ComponentRegistration, ConfigEntry, MountFn, RouteDescriptor,
};
pub use webr_web::error::Error;
pub use webr_web::ApplicationContext;
pub use webr_web::FrameworkError;
pub use webr_web::Result;

// Re-export core config types
pub use webr_web::{ConfigLoader, LogConfig, ServerConfig};

// Re-export Inject
pub use webr_web::Inject;

// ─── Error bridge helpers ────────────────────────────────────────
//
// Infrastructure crate errors (DbError, CacheError) cannot have `From`
// impls here due to Rust's orphan rule. Macros handle conversion via
// `.map_err()`. These helpers are available for manual conversion.

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
#[doc(hidden)]
pub fn __db_error(e: webr_db::DbError) -> Error {
    Error::Database(Box::new(e))
}

#[cfg(any(
    feature = "cache-memory",
    feature = "cache-sled",
    feature = "cache-redis"
))]
#[doc(hidden)]
pub fn __cache_error(e: webr_cache::CacheError) -> Error {
    Error::Cache(Box::new(e))
}

mod auto_init;
#[doc(hidden)]
pub use auto_init::auto_init as __auto_init;

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
mod db_adapter;

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
pub mod db {
    pub use crate::db_adapter::DbPool;
    pub use webr_db::{
        get_pool, scope_txn, sea_query, sea_query_binder, set_pool, sqlx, try_get_txn,
        DatasourceConfig, DbError, DbTransaction, Driver, ExecutionBinder, Page, Pagination,
        PoolConfig, QueryBinder, Result, ScalarBinder, ScopeTxnGuard, TxnInner,
    };
}

#[cfg(any(
    feature = "cache-memory",
    feature = "cache-sled",
    feature = "cache-redis"
))]
mod cache_adapter;

#[cfg(any(
    feature = "cache-memory",
    feature = "cache-sled",
    feature = "cache-redis"
))]
pub mod cache {
    pub use crate::cache_adapter::Cache;
    pub use webr_cache::{CacheConfig, CacheError, CacheStore};

    #[cfg(feature = "cache-memory")]
    pub use webr_cache::MemoryConfig;

    #[cfg(feature = "cache-sled")]
    pub use webr_cache::SledConfig;

    #[cfg(feature = "cache-redis")]
    pub use webr_cache::{HashOps, KeyOps, ListOps, RedisConfig, SetOps};
}

pub use axum::http::HeaderMap;
pub use webr_web::extract::{Form, Header, HeaderMapExt, Json, Multipart, Path, Query};
pub use webr_web::middleware;
pub use webr_web::middleware::{
    CorsMiddleware, LoggerMiddleware, Middleware, Next, PanicRecovery, ScopedMiddleware,
    UnifiedResponse,
};
pub use webr_web::response::{FileResponse, IntoSseEventResult, SseEvent, SseResponse};
pub use webr_web::router::{IntoRoutes, WebrRouter};
pub use webr_middleware::{
    AuthError, AuthMiddleware, Authenticator, CachedBody, CachedBodyMiddleware, CurrentUser, Guard,
    GuardMiddleware,
};

pub use webr_macros::HttpError;
pub use webr_macros::{
    component, config, controller, delete, entity, get, main, patch, post, put, sql, tx,
};

pub use async_trait;
pub use axum;
pub use inventory;
pub use serde;
pub use serde_json;
pub use tokio;
pub use toml;
pub use tracing;
pub use tracing_subscriber;

// Re-export dependencies so that macro-generated code can reference them without
// requiring users to add these crates as direct dependencies.
#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
pub use webr_db::sea_query;
#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
pub use webr_db::sqlx;

/// Prelude module: `use webr::prelude::*` imports all commonly used types
pub mod prelude {
    pub use super::*;
    pub use axum::http::StatusCode;
    pub use axum::response::IntoResponse;
    pub use serde::{Deserialize, Serialize};
}
