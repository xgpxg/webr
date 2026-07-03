//! Driver-agnostic query execution layer.
//!
//! [`QueryBinder`] wraps database-specific `sqlx::QueryAs` builders behind a
//! single enum, so that proc-macro-generated code never needs `#[cfg(feature)]`
//! attributes.  All driver dispatch happens inside `webr-db`.

use std::future::Future;
use std::pin::Pin;

use crate::error::DbError;
use crate::pool::DbPool;
use crate::transaction::DbTransaction;

// ─── QueryBinder ─────────────────────────────────────────────────────

/// A database-agnostic query builder wrapping `sqlx::QueryAs` for the active driver.
///
/// Created by [`DbPool::query_as`] / [`DbTransaction::query_as`].
/// Chain `.bind(value)` calls to add parameters.
pub enum QueryBinder<'q, R> {
    #[cfg(feature = "postgres")]
    Postgres(sqlx::query::QueryAs<'q, sqlx::Postgres, R, sqlx::postgres::PgArguments>),
    #[cfg(feature = "mysql")]
    MySql(sqlx::query::QueryAs<'q, sqlx::MySql, R, sqlx::mysql::MySqlArguments>),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::query::QueryAs<'q, sqlx::Sqlite, R, sqlx::sqlite::SqliteArguments<'q>>),
}

// Per-driver `bind` impls — each only requires its own Encode + Type bounds.
#[cfg(feature = "postgres")]
impl<'q, R: Send + Unpin> QueryBinder<'q, R> {
    /// Bind a value (PostgreSQL variant).
    pub fn bind<T>(self, value: T) -> Self
    where
        T: sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + 'q,
    {
        match self {
            Self::Postgres(q) => Self::Postgres(q.bind(value)),
            #[cfg(feature = "mysql")]
            Self::MySql(q) => Self::MySql(q),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(q) => Self::Sqlite(q),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

#[cfg(all(feature = "mysql", not(feature = "postgres")))]
impl<'q, R: Send + Unpin> QueryBinder<'q, R> {
    /// Bind a value (MySQL variant, postgres not enabled).
    pub fn bind<T>(self, value: T) -> Self
    where
        T: sqlx::Encode<'q, sqlx::MySql> + sqlx::Type<sqlx::MySql> + Send + 'q,
    {
        match self {
            Self::MySql(q) => Self::MySql(q.bind(value)),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(q) => Self::Sqlite(q),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

#[cfg(all(feature = "sqlite", not(feature = "postgres"), not(feature = "mysql")))]
impl<'q, R: Send + Unpin> QueryBinder<'q, R> {
    /// Bind a value (SQLite only variant).
    pub fn bind<T>(self, value: T) -> Self
    where
        T: sqlx::Encode<'q, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + 'q,
    {
        match self {
            Self::Sqlite(q) => Self::Sqlite(q.bind(value)),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

// ─── ScalarBinder ────────────────────────────────────────────────────

/// A database-agnostic scalar query builder (for `COUNT(*)` etc.).
pub enum ScalarBinder<'q, T> {
    #[cfg(feature = "postgres")]
    Postgres(sqlx::query::QueryScalar<'q, sqlx::Postgres, T, sqlx::postgres::PgArguments>),
    #[cfg(feature = "mysql")]
    MySql(sqlx::query::QueryScalar<'q, sqlx::MySql, T, sqlx::mysql::MySqlArguments>),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::query::QueryScalar<'q, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'q>>),
}

#[cfg(feature = "postgres")]
impl<'q, T: Send + Unpin> ScalarBinder<'q, T> {
    pub fn bind<V>(self, value: V) -> Self
    where
        V: sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + 'q,
    {
        match self {
            Self::Postgres(q) => Self::Postgres(q.bind(value)),
            #[cfg(feature = "mysql")]
            Self::MySql(q) => Self::MySql(q),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(q) => Self::Sqlite(q),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

#[cfg(all(feature = "mysql", not(feature = "postgres")))]
impl<'q, T: Send + Unpin> ScalarBinder<'q, T> {
    pub fn bind<V>(self, value: V) -> Self
    where
        V: sqlx::Encode<'q, sqlx::MySql> + sqlx::Type<sqlx::MySql> + Send + 'q,
    {
        match self {
            Self::MySql(q) => Self::MySql(q.bind(value)),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(q) => Self::Sqlite(q),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

#[cfg(all(feature = "sqlite", not(feature = "postgres"), not(feature = "mysql")))]
impl<'q, T: Send + Unpin> ScalarBinder<'q, T> {
    pub fn bind<V>(self, value: V) -> Self
    where
        V: sqlx::Encode<'q, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + 'q,
    {
        match self {
            Self::Sqlite(q) => Self::Sqlite(q.bind(value)),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

// ─── ExecutionBinder (for INSERT/UPDATE/DELETE) ─────────────────────

/// A database-agnostic query builder for non-SELECT statements.
pub enum ExecutionBinder<'q> {
    #[cfg(feature = "postgres")]
    Postgres(sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>),
    #[cfg(feature = "mysql")]
    MySql(sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>),
}

#[cfg(feature = "postgres")]
impl<'q> ExecutionBinder<'q> {
    pub fn bind<T>(self, value: T) -> Self
    where
        T: sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + 'q,
    {
        match self {
            Self::Postgres(q) => Self::Postgres(q.bind(value)),
            #[cfg(feature = "mysql")]
            Self::MySql(q) => Self::MySql(q),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(q) => Self::Sqlite(q),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

#[cfg(all(feature = "mysql", not(feature = "postgres")))]
impl<'q> ExecutionBinder<'q> {
    pub fn bind<T>(self, value: T) -> Self
    where
        T: sqlx::Encode<'q, sqlx::MySql> + sqlx::Type<sqlx::MySql> + Send + 'q,
    {
        match self {
            Self::MySql(q) => Self::MySql(q.bind(value)),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(q) => Self::Sqlite(q),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

#[cfg(all(feature = "sqlite", not(feature = "postgres"), not(feature = "mysql")))]
impl<'q> ExecutionBinder<'q> {
    pub fn bind<T>(self, value: T) -> Self
    where
        T: sqlx::Encode<'q, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + 'q,
    {
        match self {
            Self::Sqlite(q) => Self::Sqlite(q.bind(value)),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

// ─── DbPool execution methods ───────────────────────────────────────

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

impl DbPool {
    /// Create a typed query binder for this pool's driver.
    #[allow(unused_variables)]
    pub fn query_as<'q, R>(&'q self, sql: &'q str) -> QueryBinder<'q, R>
    where
        R: crate::Row,
    {
        match &self.inner {
            #[cfg(feature = "postgres")]
            crate::pool::PoolInner::Postgres(_) => {
                QueryBinder::Postgres(sqlx::query_as::<sqlx::Postgres, R>(sql))
            }
            #[cfg(feature = "mysql")]
            crate::pool::PoolInner::MySql(_) => {
                QueryBinder::MySql(sqlx::query_as::<sqlx::MySql, R>(sql))
            }
            #[cfg(feature = "sqlite")]
            crate::pool::PoolInner::Sqlite(_) => {
                QueryBinder::Sqlite(sqlx::query_as::<sqlx::Sqlite, R>(sql))
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("no database feature enabled"),
        }
    }

    /// Create a scalar query binder for this pool's driver.
    #[allow(unused_variables)]
    pub fn query_scalar<'q, T: crate::Scalar>(&'q self, sql: &'q str) -> ScalarBinder<'q, T> {
        match &self.inner {
            #[cfg(feature = "postgres")]
            crate::pool::PoolInner::Postgres(_) => {
                ScalarBinder::Postgres(sqlx::query_scalar::<sqlx::Postgres, T>(sql))
            }
            #[cfg(feature = "mysql")]
            crate::pool::PoolInner::MySql(_) => {
                ScalarBinder::MySql(sqlx::query_scalar::<sqlx::MySql, T>(sql))
            }
            #[cfg(feature = "sqlite")]
            crate::pool::PoolInner::Sqlite(_) => {
                ScalarBinder::Sqlite(sqlx::query_scalar::<sqlx::Sqlite, T>(sql))
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("no database feature enabled"),
        }
    }

    /// Create an execution query binder for this pool's driver.
    pub fn query_exec<'q>(&'q self, sql: &'q str) -> ExecutionBinder<'q> {
        match &self.inner {
            #[cfg(feature = "postgres")]
            crate::pool::PoolInner::Postgres(_) => {
                ExecutionBinder::Postgres(sqlx::query(sql))
            }
            #[cfg(feature = "mysql")]
            crate::pool::PoolInner::MySql(_) => {
                ExecutionBinder::MySql(sqlx::query(sql))
            }
            #[cfg(feature = "sqlite")]
            crate::pool::PoolInner::Sqlite(_) => {
                ExecutionBinder::Sqlite(sqlx::query(sql))
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("no database feature enabled"),
        }
    }

    /// Execute a query and return all rows.
    pub fn fetch_all<'q, R>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(QueryBinder<'q, R>) -> QueryBinder<'q, R> + Send + 'q,
    ) -> BoxFut<'q, Result<Vec<R>, DbError>>
    where
        R: crate::Row,
    {
        let binder = build(self.query_as(sql));
        Box::pin(async move {
            match binder {
                #[cfg(feature = "postgres")]
                QueryBinder::Postgres(q) => q.fetch_all(self.as_pg()).await.map_err(DbError::Sqlx),
                #[cfg(feature = "mysql")]
                QueryBinder::MySql(q) => q.fetch_all(self.as_my()).await.map_err(DbError::Sqlx),
                #[cfg(feature = "sqlite")]
                QueryBinder::Sqlite(q) => q.fetch_all(self.as_sq()).await.map_err(DbError::Sqlx),
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute a query and return an optional single row.
    pub fn fetch_optional<'q, R>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(QueryBinder<'q, R>) -> QueryBinder<'q, R> + Send + 'q,
    ) -> BoxFut<'q, Result<Option<R>, DbError>>
    where
        R: crate::Row,
    {
        let binder = build(self.query_as(sql));
        Box::pin(async move {
            match binder {
                #[cfg(feature = "postgres")]
                QueryBinder::Postgres(q) => q.fetch_optional(self.as_pg()).await.map_err(DbError::Sqlx),
                #[cfg(feature = "mysql")]
                QueryBinder::MySql(q) => q.fetch_optional(self.as_my()).await.map_err(DbError::Sqlx),
                #[cfg(feature = "sqlite")]
                QueryBinder::Sqlite(q) => q.fetch_optional(self.as_sq()).await.map_err(DbError::Sqlx),
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute a query and return exactly one row.
    pub fn fetch_one<'q, R>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(QueryBinder<'q, R>) -> QueryBinder<'q, R> + Send + 'q,
    ) -> BoxFut<'q, Result<R, DbError>>
    where
        R: crate::Row,
    {
        let binder = build(self.query_as(sql));
        Box::pin(async move {
            match binder {
                #[cfg(feature = "postgres")]
                QueryBinder::Postgres(q) => q.fetch_one(self.as_pg()).await.map_err(DbError::Sqlx),
                #[cfg(feature = "mysql")]
                QueryBinder::MySql(q) => q.fetch_one(self.as_my()).await.map_err(DbError::Sqlx),
                #[cfg(feature = "sqlite")]
                QueryBinder::Sqlite(q) => q.fetch_one(self.as_sq()).await.map_err(DbError::Sqlx),
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute a non-SELECT statement and return the number of affected rows.
    pub fn execute<'q>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(ExecutionBinder<'q>) -> ExecutionBinder<'q> + Send + 'q,
    ) -> BoxFut<'q, Result<u64, DbError>> {
        let binder = build(self.query_exec(sql));
        Box::pin(async move {
            match binder {
                #[cfg(feature = "postgres")]
                ExecutionBinder::Postgres(q) => {
                    q.execute(self.as_pg()).await.map(|r| r.rows_affected()).map_err(DbError::Sqlx)
                }
                #[cfg(feature = "mysql")]
                ExecutionBinder::MySql(q) => {
                    q.execute(self.as_my()).await.map(|r| r.rows_affected()).map_err(DbError::Sqlx)
                }
                #[cfg(feature = "sqlite")]
                ExecutionBinder::Sqlite(q) => {
                    q.execute(self.as_sq()).await.map(|r| r.rows_affected()).map_err(DbError::Sqlx)
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute a scalar query (e.g. `SELECT COUNT(*)`) and return the value.
    pub fn fetch_scalar<'q, T>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(ScalarBinder<'q, T>) -> ScalarBinder<'q, T> + Send + 'q,
    ) -> BoxFut<'q, Result<T, DbError>>
    where
        T: crate::Scalar,
    {
        let binder = build(self.query_scalar(sql));
        Box::pin(async move {
            match binder {
                #[cfg(feature = "postgres")]
                ScalarBinder::Postgres(q) => q.fetch_one(self.as_pg()).await.map_err(DbError::Sqlx),
                #[cfg(feature = "mysql")]
                ScalarBinder::MySql(q) => q.fetch_one(self.as_my()).await.map_err(DbError::Sqlx),
                #[cfg(feature = "sqlite")]
                ScalarBinder::Sqlite(q) => q.fetch_one(self.as_sq()).await.map_err(DbError::Sqlx),
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute an INSERT and return the created entity.
    ///
    /// - **PostgreSQL**: uses `RETURNING` clause via `insert_sql`.
    /// - **MySQL / SQLite**: executes `insert_sql`, then fetches by
    ///   `last_insert_id` / `last_insert_rowid` via `fetch_sql`.
    pub fn insert_fetch<'q, R>(
        &'q self,
        insert_sql: &'q str,
        fetch_sql: &'q str,
        _pk_col: &'q str,
        build_insert: impl FnOnce(ExecutionBinder<'q>) -> ExecutionBinder<'q> + Send + 'q,
    ) -> BoxFut<'q, Result<R, DbError>>
    where
        R: crate::Row,
    {
        let ins = build_insert(self.query_exec(insert_sql));
        Box::pin(async move {
            match ins {
                #[cfg(feature = "postgres")]
                ExecutionBinder::Postgres(q) => {
                    q.fetch_one(self.as_pg()).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "mysql")]
                ExecutionBinder::MySql(q) => {
                    let result = q.execute(self.as_my()).await.map_err(DbError::Sqlx)?;
                    let id = result.last_insert_id() as i64;
                    let fq = self.query_as::<R>(fetch_sql).bind(id);
                    match fq {
                        QueryBinder::MySql(q) => q.fetch_one(self.as_my()).await.map_err(DbError::Sqlx),
                        #[allow(unreachable_patterns)]
                        _ => unreachable!(),
                    }
                }
                #[cfg(feature = "sqlite")]
                ExecutionBinder::Sqlite(q) => {
                    let result = q.execute(self.as_sq()).await.map_err(DbError::Sqlx)?;
                    let id = result.last_insert_rowid();
                    let fq = self.query_as::<R>(fetch_sql).bind(id);
                    match fq {
                        QueryBinder::Sqlite(q) => q.fetch_one(self.as_sq()).await.map_err(DbError::Sqlx),
                        #[allow(unreachable_patterns)]
                        _ => unreachable!(),
                    }
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }
}

// ─── DbTransaction execution methods ─────────────────────────────────

impl DbTransaction {
    /// Create a typed query binder for this transaction's driver.
    #[allow(unused_variables)]
    pub fn query_as<'q, R>(&'q self, sql: &'q str) -> QueryBinder<'q, R>
    where
        R: crate::Row,
    {
        match self.driver() {
            #[cfg(feature = "postgres")]
            crate::pool::Driver::Postgres => {
                QueryBinder::Postgres(sqlx::query_as::<sqlx::Postgres, R>(sql))
            }
            #[cfg(feature = "mysql")]
            crate::pool::Driver::MySql => {
                QueryBinder::MySql(sqlx::query_as::<sqlx::MySql, R>(sql))
            }
            #[cfg(feature = "sqlite")]
            crate::pool::Driver::Sqlite => {
                QueryBinder::Sqlite(sqlx::query_as::<sqlx::Sqlite, R>(sql))
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("no database feature enabled"),
        }
    }

    /// Create a scalar query binder for this transaction's driver.
    #[allow(unused_variables)]
    pub fn query_scalar<'q, T: crate::Scalar>(&'q self, sql: &'q str) -> ScalarBinder<'q, T> {
        match self.driver() {
            #[cfg(feature = "postgres")]
            crate::pool::Driver::Postgres => {
                ScalarBinder::Postgres(sqlx::query_scalar::<sqlx::Postgres, T>(sql))
            }
            #[cfg(feature = "mysql")]
            crate::pool::Driver::MySql => {
                ScalarBinder::MySql(sqlx::query_scalar::<sqlx::MySql, T>(sql))
            }
            #[cfg(feature = "sqlite")]
            crate::pool::Driver::Sqlite => {
                ScalarBinder::Sqlite(sqlx::query_scalar::<sqlx::Sqlite, T>(sql))
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("no database feature enabled"),
        }
    }

    /// Create an execution query binder for this transaction's driver.
    pub fn query_exec<'q>(&'q self, sql: &'q str) -> ExecutionBinder<'q> {
        match self.driver() {
            #[cfg(feature = "postgres")]
            crate::pool::Driver::Postgres => {
                ExecutionBinder::Postgres(sqlx::query(sql))
            }
            #[cfg(feature = "mysql")]
            crate::pool::Driver::MySql => {
                ExecutionBinder::MySql(sqlx::query(sql))
            }
            #[cfg(feature = "sqlite")]
            crate::pool::Driver::Sqlite => {
                ExecutionBinder::Sqlite(sqlx::query(sql))
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("no database feature enabled"),
        }
    }

    /// Execute a query and return all rows (transaction context).
    pub fn fetch_all<'q, R>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(QueryBinder<'q, R>) -> QueryBinder<'q, R> + Send + 'q,
    ) -> BoxFut<'q, Result<Vec<R>, DbError>>
    where
        R: crate::Row,
    {
        let binder = build(self.query_as(sql));
        Box::pin(async move {
            let mut __g = self.lock().await;
            match binder {
                #[cfg(feature = "postgres")]
                QueryBinder::Postgres(q) => {
                    q.fetch_all(Self::as_pg(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "mysql")]
                QueryBinder::MySql(q) => {
                    q.fetch_all(Self::as_my(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "sqlite")]
                QueryBinder::Sqlite(q) => {
                    q.fetch_all(Self::as_sq(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute a query and return an optional single row (transaction context).
    pub fn fetch_optional<'q, R>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(QueryBinder<'q, R>) -> QueryBinder<'q, R> + Send + 'q,
    ) -> BoxFut<'q, Result<Option<R>, DbError>>
    where
        R: crate::Row,
    {
        let binder = build(self.query_as(sql));
        Box::pin(async move {
            let mut __g = self.lock().await;
            match binder {
                #[cfg(feature = "postgres")]
                QueryBinder::Postgres(q) => {
                    q.fetch_optional(Self::as_pg(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "mysql")]
                QueryBinder::MySql(q) => {
                    q.fetch_optional(Self::as_my(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "sqlite")]
                QueryBinder::Sqlite(q) => {
                    q.fetch_optional(Self::as_sq(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute a query and return exactly one row (transaction context).
    pub fn fetch_one<'q, R>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(QueryBinder<'q, R>) -> QueryBinder<'q, R> + Send + 'q,
    ) -> BoxFut<'q, Result<R, DbError>>
    where
        R: crate::Row,
    {
        let binder = build(self.query_as(sql));
        Box::pin(async move {
            let mut __g = self.lock().await;
            match binder {
                #[cfg(feature = "postgres")]
                QueryBinder::Postgres(q) => {
                    q.fetch_one(Self::as_pg(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "mysql")]
                QueryBinder::MySql(q) => {
                    q.fetch_one(Self::as_my(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "sqlite")]
                QueryBinder::Sqlite(q) => {
                    q.fetch_one(Self::as_sq(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute a non-SELECT statement and return affected rows (transaction context).
    pub fn execute<'q>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(ExecutionBinder<'q>) -> ExecutionBinder<'q> + Send + 'q,
    ) -> BoxFut<'q, Result<u64, DbError>> {
        let binder = build(self.query_exec(sql));
        Box::pin(async move {
            let mut __g = self.lock().await;
            match binder {
                #[cfg(feature = "postgres")]
                ExecutionBinder::Postgres(q) => {
                    q.execute(Self::as_pg(&mut __g)).await.map(|r| r.rows_affected()).map_err(DbError::Sqlx)
                }
                #[cfg(feature = "mysql")]
                ExecutionBinder::MySql(q) => {
                    q.execute(Self::as_my(&mut __g)).await.map(|r| r.rows_affected()).map_err(DbError::Sqlx)
                }
                #[cfg(feature = "sqlite")]
                ExecutionBinder::Sqlite(q) => {
                    q.execute(Self::as_sq(&mut __g)).await.map(|r| r.rows_affected()).map_err(DbError::Sqlx)
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute a scalar query (transaction context).
    pub fn fetch_scalar<'q, T>(
        &'q self,
        sql: &'q str,
        build: impl FnOnce(ScalarBinder<'q, T>) -> ScalarBinder<'q, T> + Send + 'q,
    ) -> BoxFut<'q, Result<T, DbError>>
    where
        T: crate::Scalar,
    {
        let binder = build(self.query_scalar(sql));
        Box::pin(async move {
            let mut __g = self.lock().await;
            match binder {
                #[cfg(feature = "postgres")]
                ScalarBinder::Postgres(q) => {
                    q.fetch_one(Self::as_pg(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "mysql")]
                ScalarBinder::MySql(q) => {
                    q.fetch_one(Self::as_my(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "sqlite")]
                ScalarBinder::Sqlite(q) => {
                    q.fetch_one(Self::as_sq(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }

    /// Execute an INSERT and return the created entity (transaction context).
    pub fn insert_fetch<'q, R>(
        &'q self,
        insert_sql: &'q str,
        fetch_sql: &'q str,
        _pk_col: &'q str,
        build_insert: impl FnOnce(ExecutionBinder<'q>) -> ExecutionBinder<'q> + Send + 'q,
    ) -> BoxFut<'q, Result<R, DbError>>
    where
        R: crate::Row,
    {
        let ins = build_insert(self.query_exec(insert_sql));
        Box::pin(async move {
            let mut __g = self.lock().await;
            match ins {
                #[cfg(feature = "postgres")]
                ExecutionBinder::Postgres(q) => {
                    q.fetch_one(Self::as_pg(&mut __g)).await.map_err(DbError::Sqlx)
                }
                #[cfg(feature = "mysql")]
                ExecutionBinder::MySql(q) => {
                    let result = q.execute(Self::as_my(&mut __g)).await.map_err(DbError::Sqlx)?;
                    let id = result.last_insert_id() as i64;
                    let fq = self.query_as::<R>(fetch_sql).bind(id);
                    match fq {
                        QueryBinder::MySql(q) => q.fetch_one(Self::as_my(&mut __g)).await.map_err(DbError::Sqlx),
                        #[allow(unreachable_patterns)]
                        _ => unreachable!(),
                    }
                }
                #[cfg(feature = "sqlite")]
                ExecutionBinder::Sqlite(q) => {
                    let result = q.execute(Self::as_sq(&mut __g)).await.map_err(DbError::Sqlx)?;
                    let id = result.last_insert_rowid();
                    let fq = self.query_as::<R>(fetch_sql).bind(id);
                    match fq {
                        QueryBinder::Sqlite(q) => q.fetch_one(Self::as_sq(&mut __g)).await.map_err(DbError::Sqlx),
                        #[allow(unreachable_patterns)]
                        _ => unreachable!(),
                    }
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!("no database feature enabled"),
            }
        })
    }
}
