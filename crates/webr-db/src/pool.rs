// When no database feature is enabled, Driver/PoolInner become uninhabited enums.
// Suppress unreachable_code warnings — this is expected for the no-feature case.
#![allow(unreachable_code)]

use std::time::Duration;

use crate::config::DatasourceConfig;
use crate::error::DbError;

/// Database driver identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    #[cfg(feature = "postgres")]
    Postgres,
    #[cfg(feature = "mysql")]
    MySql,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

/// Unified database connection pool.
///
/// Wraps sqlx connection pools behind a single type.
pub struct DbPool {
    pub(crate) inner: PoolInner,
}

pub(crate) enum PoolInner {
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
    #[cfg(feature = "mysql")]
    MySql(sqlx::MySqlPool),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
}

impl DbPool {
    /// Create a connection pool from configuration.
    pub async fn from_config(config: &DatasourceConfig) -> Result<Self, DbError> {
        #[allow(unused_variables)]
        let url = config.resolve_url()?;
        let pool_cfg = &config.pool;
        #[allow(unused_variables)]
        let connect_timeout = Duration::from_secs(pool_cfg.connect_timeout_secs);
        #[allow(unused_variables)]
        let idle_timeout = Duration::from_secs(pool_cfg.idle_timeout_secs);

        match config.driver.as_str() {
            #[cfg(feature = "postgres")]
            "postgres" => {
                let pg_pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(pool_cfg.max_connections)
                    .min_connections(pool_cfg.min_connections)
                    .acquire_timeout(connect_timeout)
                    .idle_timeout(idle_timeout)
                    .connect(&url)
                    .await
                    .map_err(DbError::Sqlx)?;
                Ok(Self {
                    inner: PoolInner::Postgres(pg_pool),
                })
            }
            #[cfg(feature = "mysql")]
            "mysql" => {
                let my_pool = sqlx::mysql::MySqlPoolOptions::new()
                    .max_connections(pool_cfg.max_connections)
                    .min_connections(pool_cfg.min_connections)
                    .acquire_timeout(connect_timeout)
                    .idle_timeout(idle_timeout)
                    .connect(&url)
                    .await
                    .map_err(DbError::Sqlx)?;
                Ok(Self {
                    inner: PoolInner::MySql(my_pool),
                })
            }
            #[cfg(feature = "sqlite")]
            "sqlite" => {
                let sq_pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(pool_cfg.max_connections)
                    .min_connections(pool_cfg.min_connections)
                    .acquire_timeout(connect_timeout)
                    .idle_timeout(idle_timeout)
                    .connect(&url)
                    .await
                    .map_err(DbError::Sqlx)?;
                Ok(Self {
                    inner: PoolInner::Sqlite(sq_pool),
                })
            }
            other => Err(DbError::Config(format!(
                "unsupported driver '{other}'"
            ))),
        }
    }

    /// Returns the current database driver.
    pub fn driver(&self) -> Driver {
        match &self.inner {
            #[cfg(feature = "postgres")]
            PoolInner::Postgres(_) => Driver::Postgres,
            #[cfg(feature = "mysql")]
            PoolInner::MySql(_) => Driver::MySql,
            #[cfg(feature = "sqlite")]
            PoolInner::Sqlite(_) => Driver::Sqlite,
            #[allow(unreachable_patterns)]
            _ => unreachable!("no database feature enabled"),
        }
    }

    /// Access the underlying PostgreSQL pool.
    /// # Panics
    /// Panics if the driver is not PostgreSQL.
    #[cfg(feature = "postgres")]
    pub fn as_pg(&self) -> &sqlx::PgPool {
        #[allow(unreachable_patterns)]
        match &self.inner {
            PoolInner::Postgres(p) => p,
            _ => panic!("DbPool is not PostgreSQL"),
        }
    }

    /// Access the underlying MySQL pool.
    /// # Panics
    /// Panics if the driver is not MySQL.
    #[cfg(feature = "mysql")]
    pub fn as_my(&self) -> &sqlx::MySqlPool {
        #[allow(unreachable_patterns)]
        match &self.inner {
            PoolInner::MySql(p) => p,
            _ => panic!("DbPool is not MySQL"),
        }
    }

    /// Access the underlying SQLite pool.
    /// # Panics
    /// Panics if the driver is not SQLite.
    #[cfg(feature = "sqlite")]
    pub fn as_sq(&self) -> &sqlx::SqlitePool {
        #[allow(unreachable_patterns)]
        match &self.inner {
            PoolInner::Sqlite(p) => p,
            _ => panic!("DbPool is not SQLite"),
        }
    }

    /// Generate a positional placeholder for the current driver.
    /// - PostgreSQL: `$1`, `$2`, ...
    /// - MySQL/SQLite: `?`
    #[allow(unused_variables)]
    pub fn placeholder(&self, index: usize) -> String {
        match self.driver() {
            #[cfg(feature = "postgres")]
            Driver::Postgres => format!("${index}"),
            #[cfg(feature = "mysql")]
            Driver::MySql => "?".to_string(),
            #[cfg(feature = "sqlite")]
            Driver::Sqlite => "?".to_string(),
        }
    }
}

impl std::fmt::Debug for DbPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbPool")
            .field("driver", &self.driver())
            .finish()
    }
}
