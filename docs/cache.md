# 缓存模块

支持 Memory（moka）、Sled 和 Redis 三种缓存后端，提供统一的 KV 操作 API。Redis 后端额外支持哈希、列表、集合等数据结构操作。

## 启用

```toml
[dependencies]
webr = { version = "0.1", features = ["cache-memory"] }  # 或 cache-sled, cache-redis
```

## 配置

`config/application.toml`：

```toml
[cache]
backend = "memory"          # 后端：memory / sled / redis
default_ttl = 300           # 全局默认 TTL（秒），0 表示永不过期
```

### Memory 后端配置

```toml
[cache]
backend = "memory"

[cache.memory]
max_capacity = 10000        # 最大条目数，默认 10000
time_to_idle = 0            # 空闲过期（秒），0 表示不过期
```

### Sled 后端配置

```toml
[cache]
backend = "sled"

[cache.sled]
path = "./data/cache"       # 数据库文件路径，默认 "./data/cache"
cleanup_interval = 60       # 过期键清理间隔（秒），默认 60
```

### Redis 后端配置

```toml
[cache]
backend = "redis"

[cache.redis]
url = "redis://127.0.0.1:6379"         # 连接地址，支持 cluster：逗号分隔多个 URL
key_prefix = "myapp:"                   # 键前缀，默认 ""
```

## 初始化

### auto-init

启用 `auto-init` feature 后，框架自动检测 `[cache]` 配置节并初始化：

```toml
webr = { features = ["cache-memory", "auto-init"] }
```

### 手动初始化

```rust
use webr::cache::{Cache, CacheConfig};

let config = app.config()
    .get::<CacheConfig>("cache")
    .map_err(|e| Error::Internal(e.to_string()))?;

let cache = Cache::from_config(&config).await
    .map_err(|e| Error::Cache(Box::new(e)))?;

app.provide(cache)?;
```

## 使用（KV 操作）

通过 DI 注入 `Cache` 组件：

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

        // 优先从缓存读取
        if let Some(user) = self.cache.get::<User>(&key).await.unwrap_or(None) {
            return Ok(Json(user));
        }

        // 模拟数据库查询
        let user = self.find_user(id).await?;

        // 写入缓存，TTL 60 秒
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

### KV 操作 API

| 方法 | 说明 | 所有后端 |
|------|------|----------|
| `get<T>(key)` | 读取并反序列化，不存在或已过期返回 `None` | 是 |
| `set<T>(key, value, ttl_secs)` | 序列化并写入，`ttl_secs=None` 使用全局默认 TTL | 是 |
| `del(key)` | 删除键，返回是否删除成功 | 是 |
| `exists(key)` | 检查键是否存在 | 是 |
| `clear()` | 清除所有键 | 是 |

## Redis 数据结构操作

启用 `cache-redis` feature 后，额外支持：

### Hash 操作

```rust
let hash = self.cache.hash()?;
hash.hset("user:1", "name", b"Alice").await?;
let name = hash.hget("user:1", "name").await?;
let all = hash.hget_all("user:1").await?;
hash.hdel("user:1", &["name", "email"]).await?;
let exists = hash.hexists("user:1", "name").await?;
let len = hash.hlen("user:1").await?;
```

### List 操作

```rust
let list = self.cache.list()?;
list.lpush("queue", b"task-1").await?;
list.rpush("queue", b"task-2").await?;
let task = list.lpop("queue").await?;
let task = list.rpop("queue").await?;
let len = list.llen("queue").await?;
let range = list.lrange("queue", 0, -1).await?;
```

### Set 操作

```rust
let set = self.cache.sets()?;
set.sadd("tags", &[b"rust", b"web"]).await?;
set.srem("tags", &[b"web"]).await?;
let members = set.smembers("tags").await?;
let is_member = set.sismember("tags", b"rust").await?;
let count = set.scard("tags").await?;
```

### Key 管理

```rust
let key_ops = self.cache.key()?;
key_ops.expire("temp-key", 60).await?;     // 设置过期时间
let ttl = key_ops.ttl("temp-key").await?;  // 查询剩余 TTL（-1 永不过期，-2 不存在）
```

## 多后端对比

| 特性 | Memory (moka) | Sled | Redis |
|------|---------------|------|-------|
| 外部依赖 | 无 | 无 | 需 Redis 服务 |
| 数据持久化 | 否 | 是 | 是 |
| 进程间共享 | 否 | 否（单进程） | 是 |
| 数据结构 | KV 仅 | KV | KV + Hash/List/Set |
| 性能 | 极高 | 高 | 网络延迟影响 |
| 适用场景 | 单实例缓存 | 嵌入式持久化 | 分布式缓存 |
