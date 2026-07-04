use serde::Deserialize;

/// Cache configuration, parsed from `[cache]` section of `application.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    /// Backend type: "memory", "sled", or "redis". Default: "memory"
    #[serde(default = "default_type")]
    pub backend: String,

    /// Global default TTL in seconds. 0 means no expiration.
    #[serde(default)]
    pub default_ttl: u64,

    #[cfg(feature = "memory")]
    pub memory: Option<MemoryConfig>,

    #[cfg(feature = "sled-backend")]
    pub sled: Option<SledConfig>,

    #[cfg(feature = "redis-backend")]
    pub redis: Option<RedisConfig>,
}

fn default_type() -> String {
    "memory".into()
}

/// Memory cache configuration (backed by moka).
#[cfg(feature = "memory")]
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryConfig {
    /// Max number of entries. Default: 10000
    #[serde(default = "default_max_capacity")]
    pub max_capacity: u64,
    /// Time-to-idle in seconds. 0 means no idle expiration.
    #[serde(default)]
    pub time_to_idle: u64,
}

#[cfg(feature = "memory")]
fn default_max_capacity() -> u64 {
    10_000
}

/// Sled cache configuration.
#[cfg(feature = "sled-backend")]
#[derive(Debug, Clone, Deserialize)]
pub struct SledConfig {
    /// Database file path. Default: "./data/cache"
    #[serde(default = "default_sled_path")]
    pub path: String,
    /// Interval in seconds to scan and clean expired keys. Default: 60
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval: u64,
}

#[cfg(feature = "sled-backend")]
fn default_sled_path() -> String {
    "./data/cache".into()
}

#[cfg(feature = "sled-backend")]
fn default_cleanup_interval() -> u64 {
    60
}

/// Redis cache configuration.
///
/// Supports both single-node and cluster deployments:
/// - Single node: `url = "redis://127.0.0.1:6379"`
/// - Cluster:     `url = "redis://node1:6379,redis://node2:6379,redis://node3:6379"`
///
/// The mode is auto-detected: a single URL connects in standalone mode,
/// comma-separated URLs connect in cluster mode.
#[cfg(feature = "redis-backend")]
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL(s). Comma-separated for cluster. Default: "redis://127.0.0.1:6379"
    #[serde(default = "default_redis_url")]
    pub url: String,
    /// Key prefix for all keys. Default: "" (empty)
    #[serde(default)]
    pub key_prefix: String,
}

#[cfg(feature = "redis-backend")]
fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".into()
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            backend: default_type(),
            default_ttl: 0,
            #[cfg(feature = "memory")]
            memory: None,
            #[cfg(feature = "sled-backend")]
            sled: None,
            #[cfg(feature = "redis-backend")]
            redis: None,
        }
    }
}
