//! webr-db: Database connection pool and ORM support for WebR.
//!
//! Provides `DbPool` (a unified connection pool for MySQL, PostgreSQL, SQLite)
//! that integrates with WebR's DI container via `Inject<DbPool>`.

mod config;
mod error;
mod pool;
mod transaction;

pub use config::{DatasourceConfig, PoolConfig};
pub use error::DbError;
pub use pool::{DbPool, Driver};
pub use transaction::{scope_txn, try_get_txn, DbTransaction, ScopeTxnGuard, TxnInner};

// Re-export sqlx and sea-query so downstream crates can use them
// without adding separate dependencies.
pub use sqlx;
pub use sea_query;
pub use sea_query_binder;
