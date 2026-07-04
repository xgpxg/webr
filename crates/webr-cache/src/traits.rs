use crate::error::CacheError;

/// Core KV cache interface, implemented by all backends.
#[async_trait::async_trait]
pub trait CacheStore: Send + Sync + 'static {
    /// Get raw bytes for `key`. Returns `None` if missing or expired.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;

    /// Set `key` to `value`. `ttl_secs = None` means use store default TTL.
    async fn set(&self, key: &str, value: &[u8], ttl_secs: Option<u64>) -> Result<(), CacheError>;

    /// Delete `key`. Returns `true` if the key existed.
    async fn del(&self, key: &str) -> Result<bool, CacheError>;

    /// Check if `key` exists.
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;

    /// Clear all keys in the cache namespace.
    async fn clear(&self) -> Result<(), CacheError>;
}

#[cfg(feature = "redis-backend")]
#[async_trait::async_trait]
pub trait HashOps: Send + Sync + 'static {
    async fn hset(&self, key: &str, field: &str, value: &[u8]) -> Result<(), CacheError>;
    async fn hget(&self, key: &str, field: &str) -> Result<Option<Vec<u8>>, CacheError>;
    async fn hget_all(&self, key: &str) -> Result<Vec<(String, Vec<u8>)>, CacheError>;
    async fn hdel(&self, key: &str, fields: &[&str]) -> Result<u64, CacheError>;
    async fn hexists(&self, key: &str, field: &str) -> Result<bool, CacheError>;
    async fn hlen(&self, key: &str) -> Result<u64, CacheError>;
}

#[cfg(feature = "redis-backend")]
#[async_trait::async_trait]
pub trait ListOps: Send + Sync + 'static {
    async fn lpush(&self, key: &str, value: &[u8]) -> Result<u64, CacheError>;
    async fn rpush(&self, key: &str, value: &[u8]) -> Result<u64, CacheError>;
    async fn lpop(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;
    async fn rpop(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;
    async fn llen(&self, key: &str) -> Result<u64, CacheError>;
    async fn lrange(&self, key: &str, start: i64, stop: i64) -> Result<Vec<Vec<u8>>, CacheError>;
}

#[cfg(feature = "redis-backend")]
#[async_trait::async_trait]
pub trait SetOps: Send + Sync + 'static {
    async fn sadd(&self, key: &str, members: &[&[u8]]) -> Result<u64, CacheError>;
    async fn srem(&self, key: &str, members: &[&[u8]]) -> Result<u64, CacheError>;
    async fn smembers(&self, key: &str) -> Result<Vec<Vec<u8>>, CacheError>;
    async fn sismember(&self, key: &str, member: &[u8]) -> Result<bool, CacheError>;
    async fn scard(&self, key: &str) -> Result<u64, CacheError>;
}

#[cfg(feature = "redis-backend")]
#[async_trait::async_trait]
pub trait KeyOps: Send + Sync + 'static {
    async fn expire(&self, key: &str, secs: u64) -> Result<bool, CacheError>;
    async fn ttl(&self, key: &str) -> Result<i64, CacheError>;
}
