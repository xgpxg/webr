use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};

use crate::{config::CacheConfig, error::CacheError, traits::CacheStore};

#[cfg(feature = "redis-backend")]
use crate::{redis_store::RedisStore, traits::*};

/// 统一缓存组件，通过 DI 注入使用（`Inject<Cache>`）。
///
/// 所有后端均支持泛型 KV 操作；Redis 后端额外提供哈希 / 列表 / 集合 / 键管理
/// 等数据结构操作，需启用 `redis-backend` feature。
pub struct Cache {
    store: Arc<dyn CacheStore>,
    #[cfg(feature = "redis-backend")]
    redis: Option<Arc<RedisStore>>,
}

impl Cache {
    /// 根据配置构建 `Cache` 实例，按 `backend` 字段选择后端。
    pub async fn from_config(config: &CacheConfig) -> Result<Self, CacheError> {
        match config.backend.as_str() {
            #[cfg(feature = "memory")]
            "memory" => {
                let store = Arc::new(crate::memory::MemoryStore::new(config)?);
                Ok(Self {
                    store,
                    #[cfg(feature = "redis-backend")]
                    redis: None,
                })
            }

            #[cfg(feature = "sled-backend")]
            "sled" => {
                let store = Arc::new(crate::sled_store::SledStore::new(config)?);
                Ok(Self {
                    store,
                    #[cfg(feature = "redis-backend")]
                    redis: None,
                })
            }

            #[cfg(feature = "redis-backend")]
            "redis" => {
                let store = Arc::new(RedisStore::new(config).await?);
                Ok(Self {
                    store: store.clone() as Arc<dyn CacheStore>,
                    redis: Some(store),
                })
            }

            other => Err(CacheError::Config(format!(
                "unsupported cache backend '{other}'"
            ))),
        }
    }

    // ─── 泛型 KV 操作（所有后端） ────────────────────────────

    /// 读取并反序列化缓存值。键不存在或已过期时返回 `None`。
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        let bytes = self.store.get(key).await?;
        match bytes {
            Some(b) => serde_json::from_slice(&b).map(Some).map_err(Into::into),
            None => Ok(None),
        }
    }

    /// 序列化并写入缓存值，`ttl_secs` 指定过期时间（秒）。
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: Option<u64>,
    ) -> Result<(), CacheError> {
        let bytes = serde_json::to_vec(value)?;
        self.store.set(key, &bytes, ttl_secs).await
    }

    /// 删除键，返回是否实际删除。
    pub async fn del(&self, key: &str) -> Result<bool, CacheError> {
        self.store.del(key).await
    }

    /// 检查键是否存在。
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        self.store.exists(key).await
    }

    /// 清除当前命名空间下的所有键。
    pub async fn clear(&self) -> Result<(), CacheError> {
        self.store.clear().await
    }

    // ─── Redis 专属扩展操作 ─────────────────────────────────

    /// 获取 Redis 哈希操作接口（hset / hget / hgetall / hdel / hexists / hlen）。
    #[cfg(feature = "redis-backend")]
    pub fn hash(&self) -> Result<&dyn HashOps, CacheError> {
        self.redis.as_deref().map(|r| r as &dyn HashOps).ok_or_else(|| {
            CacheError::Config("hash operations require the redis-backend feature and backend = \"redis\"".into())
        })
    }

    /// 获取 Redis 列表/队列操作接口（lpush / rpush / lpop / rpop / llen / lrange）。
    #[cfg(feature = "redis-backend")]
    pub fn list(&self) -> Result<&dyn ListOps, CacheError> {
        self.redis.as_deref().map(|r| r as &dyn ListOps).ok_or_else(|| {
            CacheError::Config("list operations require the redis-backend feature and backend = \"redis\"".into())
        })
    }

    /// 获取 Redis 集合操作接口（sadd / srem / smembers / sismember / scard）。
    #[cfg(feature = "redis-backend")]
    pub fn sets(&self) -> Result<&dyn SetOps, CacheError> {
        self.redis.as_deref().map(|r| r as &dyn SetOps).ok_or_else(|| {
            CacheError::Config("set operations require the redis-backend feature and backend = \"redis\"".into())
        })
    }

    /// 获取 Redis 键管理操作接口（expire / ttl）。
    #[cfg(feature = "redis-backend")]
    pub fn key(&self) -> Result<&dyn KeyOps, CacheError> {
        self.redis.as_deref().map(|r| r as &dyn KeyOps).ok_or_else(|| {
            CacheError::Config("key operations require the redis-backend feature and backend = \"redis\"".into())
        })
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache").finish()
    }
}
