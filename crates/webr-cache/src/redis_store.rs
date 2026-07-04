#[cfg(feature = "redis-backend")]
mod inner {
    use redis::aio::ConnectionManager;
    use redis::cluster_async::ClusterConnection;
    use redis::Client;

    use crate::{config::CacheConfig, error::CacheError, traits::*};

    /// Internal connection type: standalone or cluster.
    enum RedisConn {
        Standalone(ConnectionManager),
        Cluster(ClusterConnection),
    }

    /// Redis cache backend supporting both single-node and cluster deployments.
    ///
    /// Mode is determined by the `url` config field:
    /// - Single URL (e.g. `redis://127.0.0.1:6379`) → standalone
    /// - Comma-separated URLs (e.g. `redis://node1:6379,redis://node2:6379`) → cluster
    pub struct RedisStore {
        conn: RedisConn,
        key_prefix: String,
        default_ttl: u64,
    }

    /// Dispatches a `redis::Cmd` to the active connection variant.
    /// Return type `R` is inferred from the surrounding type annotation.
    macro_rules! run_cmd {
        ($self:expr, $cmd:expr) => {{
            match &$self.conn {
                RedisConn::Standalone(c) => {
                    let mut conn = c.clone();
                    $cmd.query_async(&mut conn)
                        .await
                        .map_err(|e| CacheError::Backend(e.to_string()))
                }
                RedisConn::Cluster(c) => {
                    let mut conn = c.clone();
                    $cmd.query_async(&mut conn)
                        .await
                        .map_err(|e| CacheError::Backend(e.to_string()))
                }
            }
        }};
    }

    impl RedisStore {
        pub async fn new(config: &CacheConfig) -> Result<Self, CacheError> {
            let redis_cfg = config
                .redis
                .as_ref()
                .ok_or_else(|| CacheError::Config("missing [cache.redis] section".into()))?;

            let urls: Vec<&str> = redis_cfg.url.split(',').map(|s| s.trim()).collect();

            let conn = if urls.len() <= 1 {
                // Single-node mode
                let client =
                    Client::open(urls[0]).map_err(|e| CacheError::Config(e.to_string()))?;
                let mgr = ConnectionManager::new(client)
                    .await
                    .map_err(|e| CacheError::Backend(e.to_string()))?;
                RedisConn::Standalone(mgr)
            } else {
                // Cluster mode
                let client = redis::cluster::ClusterClient::new(urls)
                    .map_err(|e| CacheError::Config(e.to_string()))?;
                let cluster_conn = client
                    .get_async_connection()
                    .await
                    .map_err(|e| CacheError::Backend(e.to_string()))?;
                RedisConn::Cluster(cluster_conn)
            };

            Ok(Self {
                conn,
                key_prefix: redis_cfg.key_prefix.clone(),
                default_ttl: config.default_ttl,
            })
        }

        fn full_key(&self, key: &str) -> String {
            if self.key_prefix.is_empty() {
                key.to_string()
            } else {
                format!("{}:{}", self.key_prefix, key)
            }
        }

        fn effective_ttl(&self, ttl_secs: Option<u64>) -> Option<u64> {
            match ttl_secs {
                Some(v) if v > 0 => Some(v),
                None if self.default_ttl > 0 => Some(self.default_ttl),
                _ => None,
            }
        }

        async fn set_with_ttl(
            &self,
            key: &str,
            value: &[u8],
            ttl_secs: Option<u64>,
        ) -> Result<(), CacheError> {
            let fk = self.full_key(key);
            match self.effective_ttl(ttl_secs) {
                Some(secs) => {
                    let _: () = run_cmd!(
                        self,
                        redis::cmd("SET").arg(&fk).arg(value).arg("EX").arg(secs)
                    )?;
                }
                _ => {
                    let _: () = run_cmd!(self, redis::cmd("SET").arg(&fk).arg(value))?;
                }
            }
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl CacheStore for RedisStore {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            let fk = self.full_key(key);
            let val: Option<Vec<u8>> = run_cmd!(self, redis::cmd("GET").arg(&fk))?;
            Ok(val)
        }

        async fn set(
            &self,
            key: &str,
            value: &[u8],
            ttl_secs: Option<u64>,
        ) -> Result<(), CacheError> {
            self.set_with_ttl(key, value, ttl_secs).await
        }

        async fn del(&self, key: &str) -> Result<bool, CacheError> {
            let fk = self.full_key(key);
            let removed: u64 = run_cmd!(self, redis::cmd("DEL").arg(&fk))?;
            Ok(removed > 0)
        }

        async fn exists(&self, key: &str) -> Result<bool, CacheError> {
            let fk = self.full_key(key);
            let val: bool = run_cmd!(self, redis::cmd("EXISTS").arg(&fk))?;
            Ok(val)
        }

        async fn clear(&self) -> Result<(), CacheError> {
            if self.key_prefix.is_empty() {
                match &self.conn {
                    RedisConn::Standalone(_) => {
                        let _: () = run_cmd!(self, redis::cmd("FLUSHDB"))?;
                    }
                    RedisConn::Cluster(_) => {
                        return Err(CacheError::Config(
                            "clear() without key_prefix is not supported in cluster mode; \
                             configure a key_prefix to scope key deletion"
                                .into(),
                        ));
                    }
                }
            } else {
                // Use SCAN instead of KEYS to avoid blocking Redis
                let pattern = format!("{}:*", self.key_prefix);
                let mut cursor: Vec<u8> = b"0".to_vec();
                loop {
                    let (next_cursor, keys): (Vec<u8>, Vec<String>) = match &self.conn {
                        RedisConn::Standalone(c) => {
                            let mut conn = c.clone();
                            redis::cmd("SCAN")
                                .arg(&cursor)
                                .arg("MATCH")
                                .arg(&pattern)
                                .arg("COUNT")
                                .arg(100)
                                .query_async(&mut conn)
                                .await
                                .map_err(|e| CacheError::Backend(e.to_string()))?
                        }
                        RedisConn::Cluster(c) => {
                            let mut conn = c.clone();
                            redis::cmd("SCAN")
                                .arg(&cursor)
                                .arg("MATCH")
                                .arg(&pattern)
                                .arg("COUNT")
                                .arg(100)
                                .query_async(&mut conn)
                                .await
                                .map_err(|e| CacheError::Backend(e.to_string()))?
                        }
                    };
                    if !keys.is_empty() {
                        let _: u64 = run_cmd!(self, redis::cmd("DEL").arg(&keys))?;
                    }
                    cursor = next_cursor;
                    if cursor == b"0" {
                        break;
                    }
                }
            }
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl HashOps for RedisStore {
        async fn hset(&self, key: &str, field: &str, value: &[u8]) -> Result<(), CacheError> {
            let fk = self.full_key(key);
            let _: () = run_cmd!(self, redis::cmd("HSET").arg(&fk).arg(field).arg(value))?;
            Ok(())
        }

        async fn hget(&self, key: &str, field: &str) -> Result<Option<Vec<u8>>, CacheError> {
            let fk = self.full_key(key);
            let val: Option<Vec<u8>> = run_cmd!(self, redis::cmd("HGET").arg(&fk).arg(field))?;
            Ok(val)
        }

        async fn hget_all(&self, key: &str) -> Result<Vec<(String, Vec<u8>)>, CacheError> {
            let fk = self.full_key(key);
            let val: Vec<(String, Vec<u8>)> = run_cmd!(self, redis::cmd("HGETALL").arg(&fk))?;
            Ok(val)
        }

        async fn hdel(&self, key: &str, fields: &[&str]) -> Result<u64, CacheError> {
            let fk = self.full_key(key);
            let removed: u64 = run_cmd!(self, redis::cmd("HDEL").arg(&fk).arg(fields))?;
            Ok(removed)
        }

        async fn hexists(&self, key: &str, field: &str) -> Result<bool, CacheError> {
            let fk = self.full_key(key);
            let val: bool = run_cmd!(self, redis::cmd("HEXISTS").arg(&fk).arg(field))?;
            Ok(val)
        }

        async fn hlen(&self, key: &str) -> Result<u64, CacheError> {
            let fk = self.full_key(key);
            let val: u64 = run_cmd!(self, redis::cmd("HLEN").arg(&fk))?;
            Ok(val)
        }
    }

    #[async_trait::async_trait]
    impl ListOps for RedisStore {
        async fn lpush(&self, key: &str, value: &[u8]) -> Result<u64, CacheError> {
            let fk = self.full_key(key);
            let len: u64 = run_cmd!(self, redis::cmd("LPUSH").arg(&fk).arg(value))?;
            Ok(len)
        }

        async fn rpush(&self, key: &str, value: &[u8]) -> Result<u64, CacheError> {
            let fk = self.full_key(key);
            let len: u64 = run_cmd!(self, redis::cmd("RPUSH").arg(&fk).arg(value))?;
            Ok(len)
        }

        async fn lpop(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            let fk = self.full_key(key);
            let val: Option<Vec<u8>> = run_cmd!(self, redis::cmd("LPOP").arg(&fk))?;
            Ok(val)
        }

        async fn rpop(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            let fk = self.full_key(key);
            let val: Option<Vec<u8>> = run_cmd!(self, redis::cmd("RPOP").arg(&fk))?;
            Ok(val)
        }

        async fn llen(&self, key: &str) -> Result<u64, CacheError> {
            let fk = self.full_key(key);
            let len: u64 = run_cmd!(self, redis::cmd("LLEN").arg(&fk))?;
            Ok(len)
        }

        async fn lrange(
            &self,
            key: &str,
            start: i64,
            stop: i64,
        ) -> Result<Vec<Vec<u8>>, CacheError> {
            let fk = self.full_key(key);
            let val: Vec<Vec<u8>> =
                run_cmd!(self, redis::cmd("LRANGE").arg(&fk).arg(start).arg(stop))?;
            Ok(val)
        }
    }

    #[async_trait::async_trait]
    impl SetOps for RedisStore {
        async fn sadd(&self, key: &str, members: &[&[u8]]) -> Result<u64, CacheError> {
            let fk = self.full_key(key);
            let added: u64 = run_cmd!(self, redis::cmd("SADD").arg(&fk).arg(members))?;
            Ok(added)
        }

        async fn srem(&self, key: &str, members: &[&[u8]]) -> Result<u64, CacheError> {
            let fk = self.full_key(key);
            let removed: u64 = run_cmd!(self, redis::cmd("SREM").arg(&fk).arg(members))?;
            Ok(removed)
        }

        async fn smembers(&self, key: &str) -> Result<Vec<Vec<u8>>, CacheError> {
            let fk = self.full_key(key);
            let val: Vec<Vec<u8>> = run_cmd!(self, redis::cmd("SMEMBERS").arg(&fk))?;
            Ok(val)
        }

        async fn sismember(&self, key: &str, member: &[u8]) -> Result<bool, CacheError> {
            let fk = self.full_key(key);
            let val: bool = run_cmd!(self, redis::cmd("SISMEMBER").arg(&fk).arg(member))?;
            Ok(val)
        }

        async fn scard(&self, key: &str) -> Result<u64, CacheError> {
            let fk = self.full_key(key);
            let val: u64 = run_cmd!(self, redis::cmd("SCARD").arg(&fk))?;
            Ok(val)
        }
    }

    #[async_trait::async_trait]
    impl KeyOps for RedisStore {
        async fn expire(&self, key: &str, secs: u64) -> Result<bool, CacheError> {
            let fk = self.full_key(key);
            let val: bool = run_cmd!(self, redis::cmd("EXPIRE").arg(&fk).arg(secs))?;
            Ok(val)
        }

        async fn ttl(&self, key: &str) -> Result<i64, CacheError> {
            let fk = self.full_key(key);
            let val: i64 = run_cmd!(self, redis::cmd("TTL").arg(&fk))?;
            Ok(val)
        }
    }
}

#[cfg(feature = "redis-backend")]
pub use inner::RedisStore;
