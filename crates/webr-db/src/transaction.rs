// When no database feature is enabled, Driver/TxnInner become uninhabited enums.
// Suppress unreachable_code/unreachable_patterns warnings — expected for the no-feature case.
#![allow(unreachable_code, unreachable_patterns, unused_variables)]

use std::sync::Arc;

use crate::error::DbError;
use crate::pool::{DbPool, Driver};

tokio::task_local! {
    /// Active transaction for the current async task.
    /// Set by `scope_txn`; read by generated `#[sql]` / `#[entity]` code.
    static CURRENT_TXN: std::cell::RefCell<Option<DbTransaction>>;
}

/// The raw sqlx transaction for a specific database driver.
/// Not constructed by users — created internally by `DbTransaction::begin`.
pub enum TxnInner {
    #[cfg(feature = "postgres")]
    Postgres(sqlx::Transaction<'static, sqlx::Postgres>),
    #[cfg(feature = "mysql")]
    MySql(sqlx::Transaction<'static, sqlx::MySql>),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::Transaction<'static, sqlx::Sqlite>),
}

/// An active database transaction.
///
/// Wraps a sqlx transaction behind `Arc<tokio::sync::Mutex<…>>` so that:
/// - Generated query code can obtain `&mut Connection` across `.await` points.
/// - `Clone` is cheap (Arc bump) — used internally by `scope_txn`.
#[derive(Clone)]
pub struct DbTransaction {
    inner: Arc<tokio::sync::Mutex<TxnInner>>,
    driver: Driver,
}

impl DbTransaction {
    /// Begin a new transaction from the given connection pool.
    pub async fn begin(pool: &DbPool) -> Result<Self, DbError> {
        let (inner, driver) = match pool.driver() {
            #[cfg(feature = "postgres")]
            Driver::Postgres => {
                let tx = pool.as_pg().begin().await.map_err(DbError::Sqlx)?;
                (TxnInner::Postgres(tx), Driver::Postgres)
            }
            #[cfg(feature = "mysql")]
            Driver::MySql => {
                let tx = pool.as_my().begin().await.map_err(DbError::Sqlx)?;
                (TxnInner::MySql(tx), Driver::MySql)
            }
            #[cfg(feature = "sqlite")]
            Driver::Sqlite => {
                let tx = pool.as_sq().begin().await.map_err(DbError::Sqlx)?;
                (TxnInner::Sqlite(tx), Driver::Sqlite)
            }
            _ => unreachable!("no database feature enabled"),
        };
        Ok(Self {
            inner: Arc::new(tokio::sync::Mutex::new(inner)),
            driver,
        })
    }

    /// Commit the transaction.
    pub async fn commit(self) -> Result<(), DbError> {
        let inner = Arc::try_unwrap(self.inner)
            .unwrap_or_else(|_| panic!("DbTransaction::commit called while other clones exist"))
            .into_inner();
        match inner {
            #[cfg(feature = "postgres")]
            TxnInner::Postgres(tx) => tx.commit().await.map_err(DbError::Sqlx),
            #[cfg(feature = "mysql")]
            TxnInner::MySql(tx) => tx.commit().await.map_err(DbError::Sqlx),
            #[cfg(feature = "sqlite")]
            TxnInner::Sqlite(tx) => tx.commit().await.map_err(DbError::Sqlx),
        }
    }

    /// Roll back the transaction.
    pub async fn rollback(self) -> Result<(), DbError> {
        let inner = Arc::try_unwrap(self.inner)
            .unwrap_or_else(|_| panic!("DbTransaction::rollback called while other clones exist"))
            .into_inner();
        match inner {
            #[cfg(feature = "postgres")]
            TxnInner::Postgres(tx) => tx.rollback().await.map_err(DbError::Sqlx),
            #[cfg(feature = "mysql")]
            TxnInner::MySql(tx) => tx.rollback().await.map_err(DbError::Sqlx),
            #[cfg(feature = "sqlite")]
            TxnInner::Sqlite(tx) => tx.rollback().await.map_err(DbError::Sqlx),
        }
    }

    /// The database driver this transaction was opened on.
    pub fn driver(&self) -> Driver {
        self.driver
    }

    /// Generate a positional placeholder for the current driver.
    #[allow(unused_variables)]
    pub fn placeholder(&self, index: usize) -> String {
        match self.driver {
            #[cfg(feature = "postgres")]
            Driver::Postgres => format!("${index}"),
            #[cfg(feature = "mysql")]
            Driver::MySql => "?".to_string(),
            #[cfg(feature = "sqlite")]
            Driver::Sqlite => "?".to_string(),
        }
    }

    /// Lock the inner mutex and return a guard. Use `as_pg` / `as_my` / `as_sq`
    /// on the guard to obtain a `&mut Connection`.
    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, TxnInner> {
        self.inner.lock().await
    }

    /// Obtain `&mut PgConnection` from the lock guard.
    /// # Panics
    /// Panics if the driver is not PostgreSQL.
    #[cfg(feature = "postgres")]
    pub fn as_pg(guard: &mut TxnInner) -> &mut sqlx::PgConnection {
        #[allow(unreachable_patterns)]
        match guard {
            TxnInner::Postgres(tx) => &mut **tx,
            _ => panic!("DbTransaction is not PostgreSQL"),
        }
    }

    /// Obtain `&mut MySqlConnection` from the lock guard.
    /// # Panics
    /// Panics if the driver is not MySQL.
    #[cfg(feature = "mysql")]
    pub fn as_my(guard: &mut TxnInner) -> &mut sqlx::MySqlConnection {
        #[allow(unreachable_patterns)]
        match guard {
            TxnInner::MySql(tx) => &mut **tx,
            _ => panic!("DbTransaction is not MySQL"),
        }
    }

    /// Obtain `&mut SqliteConnection` from the lock guard.
    /// # Panics
    /// Panics if the driver is not SQLite.
    #[cfg(feature = "sqlite")]
    pub fn as_sq(guard: &mut TxnInner) -> &mut sqlx::SqliteConnection {
        #[allow(unreachable_patterns)]
        match guard {
            TxnInner::Sqlite(tx) => &mut **tx,
            _ => panic!("DbTransaction is not SQLite"),
        }
    }
}

/// Check whether there is an active transaction in the current async context.
///
/// Returns `Some(&'static DbTransaction)` if a `#[tx]` scope is active,
/// `None` otherwise.
///
/// # Safety
/// The returned reference is a raw-pointer cast that is valid for the lifetime
/// of the enclosing `scope_txn` call. Generated code uses it only inside that
/// scope, so the pointer is never dangling when dereferenced.
pub fn try_get_txn() -> Option<&'static DbTransaction> {
    CURRENT_TXN
        .try_with(|cell| {
            let borrow = cell.borrow();
            if borrow.is_some() {
                // SAFETY: `scope_txn` guarantees the `DbTransaction` lives for
                // the entire `scope` future. All callers are inside that
                // future, so the pointer is valid for the duration of any single use.
                let ptr: *const DbTransaction = borrow.as_ref().unwrap();
                Some(unsafe { &*ptr })
            } else {
                None
            }
        })
        .ok()
        .flatten()
}

/// Opaque future returned by [`scope_txn`].
///
/// Awaiting this installs `txn` as the current transaction for the duration
/// of the wrapped future, then returns the result.
pub struct ScopeTxnGuard<'a, R> {
    inner: std::pin::Pin<Box<dyn std::future::Future<Output = R> + Send + 'a>>,
}

impl<R> std::future::Future for ScopeTxnGuard<'_, R> {
    type Output = R;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<R> {
        self.inner.as_mut().poll(cx)
    }
}

/// Install `txn` as the current transaction for the duration of `f`.
///
/// Returns a [`ScopeTxnGuard`] future. When awaited, `txn` is available via
/// [`try_get_txn`] to all code running inside `f`. The caller retains ownership
/// of `txn` (through the `&` reference) and can commit/rollback after the guard
/// resolves.
///
/// Used exclusively by the `#[tx]` macro — user code should not call this directly.
pub fn scope_txn<'a, R>(
    txn: &'a DbTransaction,
    f: impl std::future::Future<Output = R> + Send + 'a,
) -> ScopeTxnGuard<'a, R> {
    let txn = txn.clone();
    ScopeTxnGuard {
        inner: Box::pin(async move {
            let cell = std::cell::RefCell::new(Some(txn));
            CURRENT_TXN.scope(cell, f).await
        }),
    }
}
