pub use webr_core::app::AppBuilder;
pub use webr_core::component::{
    Component, ComponentEntry, ComponentRegistration, MountFn, RouteDescriptor,
};
pub use webr_core::config::{ConfigEntry, ConfigLoader, LogConfig, ServerConfig};
pub use webr_core::context::ApplicationContext;
pub use webr_core::error::{ValidationFieldError, Error, WebrResult};

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
mod db_adapter;

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
pub mod db {
    pub use webr_db::{
        DatasourceConfig, DbError, Driver, PoolConfig,
        scope_txn, try_get_txn, DbTransaction, ScopeTxnGuard, TxnInner,
        QueryBinder, ScalarBinder, ExecutionBinder,
        sqlx, sea_query, sea_query_binder,
    };
    pub use crate::db_adapter::DbPool;
}
pub use axum::http::HeaderMap;
pub use webr_core::extract::{Form, Header, HeaderMapExt, Json, Multipart, Path, Query};
pub use webr_core::inject::Inject;
pub use webr_core::middleware::{
    CorsMiddleware, LoggerMiddleware, Middleware, Next, PanicRecovery, ScopedMiddleware,
    UnifiedResponse,
};
pub use webr_core::response::{FileResponse, IntoSseEventResult, SseEvent, SseResponse};
pub use webr_core::router::{IntoRoutes, WebrRouter};

pub use webr_macros::HttpError;
pub use webr_macros::{
    component, config, controller, delete, entity, get, main, patch, post, put, sql, tx, Validate,
};

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
