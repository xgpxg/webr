//! DI adapter for `webr_cache::Cache`.
//!
//! `webr_cache` is kept free of framework dependencies.
//! This module bridges the gap: `Cache` here wraps `webr_cache::Cache`
//! and implements `Component` so it can be managed by the DI container.

use std::ops::Deref;

use webr_core::component::Component;

/// DI-compatible wrapper around [`webr_cache::Cache`].
///
/// Implements [`Component`] so the cache can be registered with
/// `app.provide(cache)` and injected via `Inject<Cache>`.
///
/// Derefs to the inner [`webr_cache::Cache`], so all cache methods
/// (`get`, `set`, `del`, `hash`, `list`, …) are available without
/// explicit delegation.
pub struct Cache(webr_cache::Cache);

impl Cache {
    /// Create a cache from configuration.
    pub async fn from_config(
        config: &webr_cache::CacheConfig,
    ) -> Result<Self, webr_cache::CacheError> {
        webr_cache::Cache::from_config(config).await.map(Self)
    }

    /// Access the underlying [`webr_cache::Cache`].
    pub fn inner(&self) -> &webr_cache::Cache {
        &self.0
    }
}

impl Deref for Cache {
    type Target = webr_cache::Cache;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Component for Cache {
    fn component_name() -> &'static str {
        "Cache"
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
