//! Cache module for the Webr framework.
//!
//! Provides a unified `Cache` component with three backends:
//! - **memory** (moka): high-performance in-process cache, default backend
//! - **sled**: embedded persistent cache, no external service required
//! - **redis**: distributed cache via Redis, supports extended data structures
//!
//! Extended operations (hash / list / set / key management) are only available
//! with the `redis-backend` feature.

mod cache;
mod config;
mod error;
mod traits;

#[cfg(feature = "memory")]
mod memory;

#[cfg(feature = "sled-backend")]
mod sled_store;

#[cfg(feature = "redis-backend")]
mod redis_store;

pub use cache::Cache;
pub use config::CacheConfig;
pub use error::CacheError;
pub use traits::CacheStore;

#[cfg(feature = "memory")]
pub use config::MemoryConfig;

#[cfg(feature = "sled-backend")]
pub use config::SledConfig;

#[cfg(feature = "redis-backend")]
pub use config::RedisConfig;

#[cfg(feature = "redis-backend")]
pub use traits::{HashOps, KeyOps, ListOps, SetOps};
