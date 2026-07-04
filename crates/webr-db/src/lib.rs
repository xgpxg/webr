//! Database connection pool and ORM support.

mod config;
mod error;
pub mod executor;
mod page;
mod pool;
mod row;
mod transaction;

pub use config::{DatasourceConfig, PoolConfig};
pub use error::DbError;
pub use executor::{ExecutionBinder, QueryBinder, ScalarBinder};
pub use page::{Page, Pagination};
pub use pool::{DbPool, Driver};
pub use row::{Row, Scalar};
pub use transaction::{scope_txn, try_get_txn, DbTransaction, ScopeTxnGuard, TxnInner};

// Re-export sqlx and sea-query so downstream crates can use them
// without adding separate dependencies.
pub use sea_query;
pub use sea_query_binder;
pub use sqlx;

pub type Result<T> = std::result::Result<T, DbError>;
