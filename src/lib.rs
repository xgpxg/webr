pub use webr_core::app::AppBuilder;
pub use webr_core::component::{
    Component, ComponentEntry, ComponentRegistration, MountFn, RouteDescriptor,
};
pub use webr_core::config::{ConfigEntry, ConfigLoader, LogConfig, ServerConfig};
pub use webr_core::context::ApplicationContext;
pub use webr_core::error::{Error, ValidationFieldError, WebrResult};

mod auto_init;
#[doc(hidden)]
pub use auto_init::auto_init as __auto_init;

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
mod db_adapter;

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
pub mod db {
    pub use crate::db_adapter::DbPool;
    pub use webr_db::{
        scope_txn, sea_query, sea_query_binder, sqlx, try_get_txn, DatasourceConfig, DbError,
        DbTransaction, Driver, ExecutionBinder, Page, Pagination, PoolConfig, QueryBinder,
        Result, ScalarBinder, ScopeTxnGuard, TxnInner,
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
pub use webr_core::extract::{Form, Header, HeaderMapExt, Json, Multipart, Path, Query};
pub use webr_core::inject::Inject;
pub use webr_core::middleware;
pub use webr_core::middleware::{
    CorsMiddleware, LoggerMiddleware, Middleware, Next, PanicRecovery, ScopedMiddleware,
    UnifiedResponse,
};
pub use webr_core::response::{FileResponse, IntoSseEventResult, SseEvent, SseResponse};
pub use webr_core::router::{IntoRoutes, WebrRouter};
pub use webr_middleware::{
    AuthError, AuthMiddleware, Authenticator, CachedBody, CachedBodyMiddleware, CurrentUser, Guard,
    GuardMiddleware,
};

pub use webr_macros::HttpError;
pub use webr_macros::{
    component, config, controller, delete, entity, get, main, patch, post, put, sql, tx, Validate,
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
pub use validator;

// Re-export sqlx so that #[entity] / #[sql] generated code can reference `webr::db::sqlx`
#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
pub use webr_db::sqlx;

/// Prelude 模块：`use webr::prelude::*` 导入所有常用类型
pub mod prelude {
    pub use super::*;
    pub use axum::http::StatusCode;
    pub use axum::response::IntoResponse;
    pub use serde::{Deserialize, Serialize};
}
