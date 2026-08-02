# Cache

Supports three cache backends: Memory (moka), Sled, and Redis, providing a unified KV operation API. The Redis backend additionally supports hash, list, set, and other data structure operations.

## Enable

```toml
[dependencies]
webr = { version = "0.1", features = ["cache-memory"] }  # or cache-sled, cache-redis
```

## Configuration

`config/application.toml`:

```toml
[cache]
backend = "memory"          # Backend: memory / sled / redis
default_ttl = 300           # Global default TTL (seconds), 0 means never expire
```

### Memory backend configuration

```toml
[cache]
backend = "memory"

[cache.memory]
max_capacity = 10000        # Max entries, default 10000
time_to_idle = 0            # Idle expiry (seconds), 0 means no expiry
```

### Sled backend configuration

```toml
[cache]
backend = "sled"

[cache.sled]
path = "./data/cache"       # Database file path, default "./data/cache"
cleanup_interval = 60       # Expired key cleanup interval (seconds), default 60
```

### Redis backend configuration

```toml
[cache]
backend = "redis"

[cache.redis]
url = "redis://127.0.0.1:6379"         # Connection address, supports cluster: comma-separated multiple URLs
key_prefix = "myapp:"                   # Key prefix, default ""
```

## Initialization

### auto-init

After enabling the `auto-init` feature, the framework automatically detects the `[cache]` configuration section and initializes:

```toml
webr = { features = ["cache-memory", "auto-init"] }
```

### Manual initialization

```rust
use webr::cache::{Cache, CacheConfig};

let config = app.config()
    .get::<CacheConfig>("cache")
    .map_err(|e| Error::Internal(e.to_string()))?;

let cache = Cache::from_config(&config).await
    .map_err(|e| Error::Cache(Box::new(e)))?;

app.provide(cache)?;
```

## Usage (KV operations)

Inject the `Cache` component via DI:

```rust
use webr::cache::Cache;

#[controller]
pub struct UserController {
    cache: Inject<Cache>,
}

impl UserController {
    #[get("/users/{id}")]
    async fn get_user(&self, Path(id): Path<i64>) -> Result<Json<User>> {
        let key = format!("user:{id}");

        // Read from cache first
        if let Some(user) = self.cache.get::<User>(&key).await.unwrap_or(None) {
            return Ok(Json(user));
        }

        // Simulate database query
        let user = self.find_user(id).await?;

        // Write to cache, TTL 60 seconds
        self.cache.set(&key, &user, Some(60)).await
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Json(user))
    }

    #[delete("/users/{id}")]
    async fn delete_user(&self, Path(id): Path<i64>) -> StatusCode {
        let key = format!("user:{id}");
        let _ = self.cache.del(&key).await;
        StatusCode::NO_CONTENT
    }
}
```

### KV operation API

| Method | Description | All backends |
|--------|-------------|:------------:|
| `get<T>(key)` | Read and deserialize, returns `None` if not found or expired | Yes |
| `set<T>(key, value, ttl_secs)` | Serialize and write, `ttl_secs=None` uses global default TTL | Yes |
| `del(key)` | Delete key, returns whether deletion succeeded | Yes |
| `exists(key)` | Check if key exists | Yes |
| `clear()` | Clear all keys | Yes |

## Redis data structure operations

With `cache-redis` feature enabled, additionally supports:

### Hash operations

```rust
let hash = self.cache.hash()?;
hash.hset("user:1", "name", b"Alice").await?;
let name = hash.hget("user:1", "name").await?;
let all = hash.hget_all("user:1").await?;
hash.hdel("user:1", &["name", "email"]).await?;
let exists = hash.hexists("user:1", "name").await?;
let len = hash.hlen("user:1").await?;
```

### List operations

```rust
let list = self.cache.list()?;
list.lpush("queue", b"task-1").await?;
list.rpush("queue", b"task-2").await?;
let task = list.lpop("queue").await?;
let task = list.rpop("queue").await?;
let len = list.llen("queue").await?;
let range = list.lrange("queue", 0, -1).await?;
```

### Set operations

```rust
let set = self.cache.sets()?;
set.sadd("tags", &[b"rust", b"web"]).await?;
set.srem("tags", &[b"web"]).await?;
let members = set.smembers("tags").await?;
let is_member = set.sismember("tags", b"rust").await?;
let count = set.scard("tags").await?;
```

### Key management

```rust
let key_ops = self.cache.key()?;
key_ops.expire("temp-key", 60).await?;     // Set expiry time
let ttl = key_ops.ttl("temp-key").await?;  // Query remaining TTL (-1 never expires, -2 does not exist)
```

## Backend comparison

| Feature | Memory (moka) | Sled | Redis |
|---------|---------------|------|-------|
| External dependency | None | None | Requires Redis service |
| Data persistence | No | Yes | Yes |
| Cross-process sharing | No | No (single process) | Yes |
| Data structures | KV only | KV | KV + Hash/List/Set |
| Performance | Very high | High | Affected by network latency |
| Use case | Single-instance cache | Embedded persistence | Distributed cache |
