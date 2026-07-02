use std::time::Duration;

use crate::config::DatasourceConfig;
use crate::error::DbError;

/// Database driver identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    Postgres,
    MySql,
    Sqlite,
}

/// Unified database connection pool supporting MySQL, PostgreSQL, and SQLite.
///
/// Wraps sqlx connection pools behind a single type, enabling DI injection
/// via `Inject<DbPool>` in `#[service]` structs.
pub struct DbPool {
    inner: PoolInner,
}

enum PoolInner {
    Postgres(sqlx::PgPool),
    MySql(sqlx::MySqlPool),
    Sqlite(sqlx::SqlitePool),
}

impl DbPool {
    /// Create a connection pool from configuration.
    pub async fn from_config(config: &DatasourceConfig) -> Result<Self, DbError> {
        let url = config.resolve_url()?;
        let pool_cfg = &config.pool;
        let connect_timeout = Duration::from_secs(pool_cfg.connect_timeout_secs);
        let idle_timeout = Duration::from_secs(pool_cfg.idle_timeout_secs);

        match config.driver.as_str() {
            "postgres" => {
                let pg_pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(pool_cfg.max_connections)
                    .min_connections(pool_cfg.min_connections)
                    .acquire_timeout(connect_timeout)
                    .idle_timeout(idle_timeout)
                    .connect(&url)
                    .await
                    .map_err(DbError::Sqlx)?;
                Ok(Self { inner: PoolInner::Postgres(pg_pool) })
            }
            "mysql" => {
                let my_pool = sqlx::mysql::MySqlPoolOptions::new()
                    .max_connections(pool_cfg.max_connections)
                    .min_connections(pool_cfg.min_connections)
                    .acquire_timeout(connect_timeout)
                    .idle_timeout(idle_timeout)
                    .connect(&url)
                    .await
                    .map_err(DbError::Sqlx)?;
                Ok(Self { inner: PoolInner::MySql(my_pool) })
            }
            "sqlite" => {
                let sq_pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(pool_cfg.max_connections)
                    .min_connections(pool_cfg.min_connections)
                    .acquire_timeout(connect_timeout)
                    .idle_timeout(idle_timeout)
                    .connect(&url)
                    .await
                    .map_err(DbError::Sqlx)?;
                Ok(Self { inner: PoolInner::Sqlite(sq_pool) })
            }
            other => Err(DbError::Config(
                format!("unsupported driver '{other}', expected: mysql, postgres, or sqlite"),
            )),
        }
    }

    /// Returns the current database driver.
    pub fn driver(&self) -> Driver {
        match &self.inner {
            PoolInner::Postgres(_) => Driver::Postgres,
            PoolInner::MySql(_) => Driver::MySql,
            PoolInner::Sqlite(_) => Driver::Sqlite,
        }
    }

    /// Access the underlying PostgreSQL pool.
    /// # Panics
    /// Panics if the driver is not PostgreSQL.
    pub fn as_pg(&self) -> &sqlx::PgPool {
        match &self.inner {
            PoolInner::Postgres(p) => p,
            _ => panic!("DbPool is not PostgreSQL"),
        }
    }

    /// Access the underlying MySQL pool.
    /// # Panics
    /// Panics if the driver is not MySQL.
    pub fn as_my(&self) -> &sqlx::MySqlPool {
        match &self.inner {
            PoolInner::MySql(p) => p,
            _ => panic!("DbPool is not MySQL"),
        }
    }

    /// Access the underlying SQLite pool.
    /// # Panics
    /// Panics if the driver is not SQLite.
    pub fn as_sq(&self) -> &sqlx::SqlitePool {
        match &self.inner {
            PoolInner::Sqlite(p) => p,
            _ => panic!("DbPool is not SQLite"),
        }
    }

    /// Generate a positional placeholder for the current driver.
    /// - PostgreSQL: `$1`, `$2`, ...
    /// - MySQL/SQLite: `?`
    pub fn placeholder(&self, index: usize) -> String {
        match self.driver() {
            Driver::Postgres => format!("${index}"),
            Driver::MySql | Driver::Sqlite => "?".to_string(),
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

// ─── Component 实现：使 DbPool 可被 DI 容器注入 ────────────────

impl webr_core::component::Component for DbPool {
    fn component_name() -> &'static str {
        "DbPool"
    }
}
