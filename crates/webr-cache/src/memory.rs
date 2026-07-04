#[cfg(feature = "memory")]
mod inner {
    use std::time::{Duration, Instant};

    use moka::future::Cache as MokaCache;

    use crate::{config::CacheConfig, error::CacheError, traits::CacheStore};

    /// Stored value with optional expiration timestamp.
    #[derive(Clone)]
    struct Entry {
        value: Vec<u8>,
        /// `Instant` at which this entry expires. `None` = no expiration.
        expires_at: Option<Instant>,
    }

    impl Entry {
        fn is_expired(&self) -> bool {
            self.expires_at.map_or(false, |t| Instant::now() >= t)
        }
    }

    /// In-memory cache backend powered by moka.
    pub struct MemoryStore {
        inner: MokaCache<String, Entry>,
        default_ttl: u64,
    }

    impl MemoryStore {
        pub fn new(config: &CacheConfig) -> Result<Self, CacheError> {
            let mem_cfg = config.memory.as_ref();
            let max_capacity = mem_cfg.map(|c| c.max_capacity).unwrap_or(10_000);
            let time_to_idle = mem_cfg.and_then(|c| {
                if c.time_to_idle > 0 {
                    Some(Duration::from_secs(c.time_to_idle))
                } else {
                    None
                }
            });

            let mut builder = MokaCache::builder().max_capacity(max_capacity);
            if let Some(tti) = time_to_idle {
                builder = builder.time_to_idle(tti);
            }

            Ok(Self {
                inner: builder.build(),
                default_ttl: config.default_ttl,
            })
        }

        /// Compute the expiration instant for a write.
        fn compute_expiry(&self, ttl_secs: Option<u64>) -> Option<Instant> {
            let secs = ttl_secs.or_else(|| {
                if self.default_ttl > 0 {
                    Some(self.default_ttl)
                } else {
                    None
                }
            })?;
            if secs == 0 {
                return None;
            }
            Some(Instant::now() + Duration::from_secs(secs))
        }
    }

    #[async_trait::async_trait]
    impl CacheStore for MemoryStore {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            let entry = match self.inner.get(key).await {
                Some(e) => e,
                None => return Ok(None),
            };
            if entry.is_expired() {
                self.inner.remove(key).await;
                return Ok(None);
            }
            Ok(Some(entry.value.clone()))
        }

        async fn set(
            &self,
            key: &str,
            value: &[u8],
            ttl_secs: Option<u64>,
        ) -> Result<(), CacheError> {
            let entry = Entry {
                value: value.to_vec(),
                expires_at: self.compute_expiry(ttl_secs),
            };
            self.inner.insert(key.to_string(), entry).await;
            Ok(())
        }

        async fn del(&self, key: &str) -> Result<bool, CacheError> {
            Ok(self.inner.remove(key).await.is_some())
        }

        async fn exists(&self, key: &str) -> Result<bool, CacheError> {
            match self.inner.get(key).await {
                Some(e) if !e.is_expired() => Ok(true),
                Some(_) => {
                    // Expired, lazy cleanup (consistent with `get`)
                    self.inner.remove(key).await;
                    Ok(false)
                }
                None => Ok(false),
            }
        }

        async fn clear(&self) -> Result<(), CacheError> {
            self.inner.invalidate_all();
            self.inner.run_pending_tasks().await;
            Ok(())
        }
    }
}

#[cfg(feature = "memory")]
pub use inner::MemoryStore;
