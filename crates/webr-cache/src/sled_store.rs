#[cfg(feature = "sled-backend")]
mod inner {
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use sled::Db;
    use tokio::task::JoinHandle;

    use crate::{config::CacheConfig, error::CacheError, traits::CacheStore};

    /// Sled entry format: `[1-byte flag][optional 8-byte epoch_secs][value...]`
    /// flag = 0 → no expiration
    /// flag = 1 → followed by u64 big-endian expiration epoch seconds
    const FLAG_NO_EXPIRY: u8 = 0;
    const FLAG_WITH_EXPIRY: u8 = 1;

    fn encode_entry(value: &[u8], expires_at: Option<u64>) -> Vec<u8> {
        match expires_at {
            None => {
                let mut buf = Vec::with_capacity(1 + value.len());
                buf.push(FLAG_NO_EXPIRY);
                buf.extend_from_slice(value);
                buf
            }
            Some(secs) => {
                let mut buf = Vec::with_capacity(9 + value.len());
                buf.push(FLAG_WITH_EXPIRY);
                buf.extend_from_slice(&secs.to_be_bytes());
                buf.extend_from_slice(value);
                buf
            }
        }
    }

    fn decode_entry(data: &[u8]) -> Option<(Option<u64>, &[u8])> {
        if data.is_empty() {
            return None;
        }
        match data[0] {
            FLAG_NO_EXPIRY => Some((None, &data[1..])),
            FLAG_WITH_EXPIRY if data.len() >= 9 => {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&data[1..9]);
                let expires_at = u64::from_be_bytes(buf);
                Some((Some(expires_at), &data[9..]))
            }
            _ => None,
        }
    }

    fn is_expired(expires_at: Option<u64>) -> bool {
        match expires_at {
            Some(secs) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                now >= secs
            }
            None => false,
        }
    }

    fn now_epoch() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Persistent embedded cache backend powered by sled.
    pub struct SledStore {
        db: Db,
        default_ttl: u64,
        _cleanup: Option<JoinHandle<()>>,
    }

    impl SledStore {
        pub fn new(config: &CacheConfig) -> Result<Self, CacheError> {
            let sled_cfg = config.sled.as_ref();
            let path = sled_cfg.map(|c| c.path.as_str()).unwrap_or("./data/cache");
            let cleanup_interval = sled_cfg.map(|c| c.cleanup_interval).unwrap_or(60);

            let db = sled::open(path).map_err(|e| CacheError::Config(e.to_string()))?;

            // Spawn background cleanup task for expired keys
            let db_clone = Arc::new(db.clone());
            let interval = Duration::from_secs(cleanup_interval);
            let cleanup = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(interval).await;
                    Self::sweep_expired(&db_clone);
                }
            });

            Ok(Self {
                db,
                default_ttl: config.default_ttl,
                _cleanup: Some(cleanup),
            })
        }

        fn effective_ttl(&self, ttl_secs: Option<u64>) -> Option<u64> {
            let secs = ttl_secs.or(if self.default_ttl > 0 {
                Some(self.default_ttl)
            } else {
                None
            })?;
            if secs == 0 {
                None
            } else {
                Some(now_epoch() + secs)
            }
        }

        fn sweep_expired(db: &Db) {
            let now = now_epoch();
            for entry in db.iter() {
                let Ok((key, value)) = entry else {
                    continue;
                };
                if let Some((Some(expires_at), _)) = decode_entry(&value) {
                    if now >= expires_at {
                        let _ = db.remove(key);
                    }
                }
            }
        }
    }

    impl Drop for SledStore {
        fn drop(&mut self) {
            if let Some(handle) = self._cleanup.take() {
                handle.abort();
            }
        }
    }

    #[async_trait::async_trait]
    impl CacheStore for SledStore {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            let raw = self
                .db
                .get(key)
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            let raw = match raw {
                Some(r) => r,
                None => return Ok(None),
            };
            match decode_entry(&raw) {
                Some((expires_at, value)) if !is_expired(expires_at) => Ok(Some(value.to_vec())),
                Some((Some(_), _)) => {
                    // Expired, lazy delete
                    let _ = self.db.remove(key);
                    Ok(None)
                }
                _ => Ok(None),
            }
        }

        async fn set(
            &self,
            key: &str,
            value: &[u8],
            ttl_secs: Option<u64>,
        ) -> Result<(), CacheError> {
            let expires_at = self.effective_ttl(ttl_secs);
            let entry = encode_entry(value, expires_at);
            self.db
                .insert(key, entry)
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            Ok(())
        }

        async fn del(&self, key: &str) -> Result<bool, CacheError> {
            let old = self
                .db
                .remove(key)
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            Ok(old.is_some())
        }

        async fn exists(&self, key: &str) -> Result<bool, CacheError> {
            let raw = self
                .db
                .get(key)
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            match raw {
                Some(r) => match decode_entry(&r) {
                    Some((expires_at, _)) => Ok(!is_expired(expires_at)),
                    None => Ok(false),
                },
                None => Ok(false),
            }
        }

        async fn clear(&self) -> Result<(), CacheError> {
            self.db
                .clear()
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            Ok(())
        }
    }
}

#[cfg(feature = "sled-backend")]
pub use inner::SledStore;
