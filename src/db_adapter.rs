//! DI adapter for `webr_db::DbPool`.
//!
//! `webr_db` is intentionally kept free of framework dependencies.
//! This module bridges the gap: `DbPool` here wraps `webr_db::DbPool`
//! and implements `Component` so it can be managed by the DI container.

use std::ops::Deref;

use webr_core::component::Component;

/// DI-compatible wrapper around [`webr_db::DbPool`].
///
/// Implements [`Component`] so the pool can be registered with
/// `app.provide(pool)` and injected via `Inject<DbPool>`.
///
/// Derefs to the inner [`webr_db::DbPool`], so all pool methods
/// (`as_pg`, `as_my`, `as_sq`, `driver`, `placeholder`, …) are
/// available without explicit delegation.
pub struct DbPool(webr_db::DbPool);

impl DbPool {
    /// Create a connection pool from configuration.
    pub async fn from_config(
        config: &webr_db::DatasourceConfig,
    ) -> Result<Self, webr_db::DbError> {
        webr_db::DbPool::from_config(config).await.map(Self)
    }

    /// Access the underlying [`webr_db::DbPool`].
    pub fn inner(&self) -> &webr_db::DbPool {
        &self.0
    }
}

impl Deref for DbPool {
    type Target = webr_db::DbPool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Component for DbPool {
    fn component_name() -> &'static str {
        "DbPool"
    }
}

impl std::fmt::Debug for DbPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
