//! Auto-initialization of framework components from configuration.
//!
//! This module is called automatically by `#[main]` macro before `build()`.
//! It detects configuration sections and initializes corresponding components.
//!
//! Enable with `auto-init` feature to auto-initialize cache/db from config.

use crate::{AppBuilder, Error};

/// Auto-initialize framework components (cache, db, etc.) from configuration.
/// Called automatically by `#[main]` macro before `build()`.
///
/// When `auto-init` feature is enabled, this function will:
/// - Auto-initialize cache if `[cache]` section exists in config
/// - Auto-initialize database pool if `[datasource]` section exists in config
///
/// When `auto-init` feature is disabled, this function is a no-op.
pub async fn auto_init(app: &mut AppBuilder) -> Result<(), Error> {
    #[cfg(feature = "auto-init")]
    {
        // Auto-initialize cache if configured
        #[cfg(any(feature = "cache-memory", feature = "cache-sled", feature = "cache-redis"))]
        {
            if let Ok(cache_config) = app.config().get::<webr_cache::CacheConfig>("cache") {
                let cache = crate::cache_adapter::Cache::from_config(&cache_config)
                    .await
                    .map_err(crate::__cache_error)?;
                app.provide(cache)?;
                tracing::info!("Cache auto-initialized with backend: {}", cache_config.backend);
            }
        }

        // Auto-initialize database pool if configured
        #[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
        {
            if let Ok(ds_config) = app.config().get::<webr_db::DatasourceConfig>("datasource") {
                let pool = crate::db_adapter::DbPool::from_config(&ds_config)
                    .await
                    .map_err(crate::__db_error)?;
                // Store pool globally for #[entity] generated code
                webr_db::set_pool(pool.inner().clone());
                app.provide(pool)?;
                tracing::info!(
                    "Database pool auto-initialized with driver: {}",
                    ds_config.driver
                );
            }
        }
    }

    // Suppress unused variable warning when auto-init is disabled
    #[cfg(not(feature = "auto-init"))]
    let _ = app;

    Ok(())
}
