use serde::{Deserialize, Serialize};
use webr::prelude::*;
use webr::{cache::Cache, Error, Inject};

// ─── DTO ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

// ─── Controller ───────────────────────────────────────────

#[controller]
pub struct CacheController {
    cache: Inject<Cache>,
}

#[controller]
impl CacheController {
    /// GET / — 首页
    #[get("/")]
    async fn index(&self) -> &'static str {
        "Cache Example API — try GET /users/1, POST /users, DELETE /users/1"
    }

    /// GET /health — 健康检查
    #[get("/health")]
    async fn health(&self) -> StatusCode {
        StatusCode::OK
    }

    /// GET /users/{id} — 获取用户（带缓存）
    #[get("/users/{id}")]
    async fn get_user(&self, Path(id): Path<i64>) -> Result<Json<User>> {
        let key = format!("user:{id}");

        // 优先从缓存读取
        if let Some(user) = self.cache.get::<User>(&key).await.unwrap_or(None) {
            return Ok(Json(user));
        }

        // 模拟数据库查询（实际项目中替换为真实查询）
        let user = self.mock_find_user(id).await?;

        // 写入缓存，TTL 60 秒（覆盖全局默认 TTL）
        self.cache.set(&key, &user, Some(60)).await
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Json(user))
    }

    /// POST /users — 创建用户并缓存
    #[post("/users")]
    async fn create_user(&self, Json(body): Json<CreateUserRequest>) -> Result<Json<User>> {
        // 模拟创建用户
        let user = User {
            id: 1,
            name: body.name,
            email: body.email,
        };

        // 写入缓存
        let key = format!("user:{}", user.id);
        self.cache.set(&key, &user, Some(60)).await
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Json(user))
    }

    /// DELETE /users/{id} — 删除用户并清除缓存
    #[delete("/users/{id}")]
    async fn delete_user(&self, Path(id): Path<i64>) -> StatusCode {
        let key = format!("user:{id}");
        let _ = self.cache.del(&key).await;
        StatusCode::NO_CONTENT
    }

    /// GET /cache/stats — 缓存状态
    #[get("/cache/stats")]
    async fn cache_stats(&self) -> Json<serde_json::Value> {
        // 检查特定 key 是否存在
        let user_exists = self.cache.exists("user:1").await.unwrap_or(false);

        Json(serde_json::json!({
            "user_1_cached": user_exists,
        }))
    }

    /// POST /cache/clear — 清空所有缓存
    #[post("/cache/clear")]
    async fn clear_cache(&self) -> &'static str {
        let _ = self.cache.clear().await;
        "Cache cleared"
    }

    // ─── 内部方法 ─────────────────────────────────────────

    /// 模拟数据库查询（实际项目中替换为真实查询）
    async fn mock_find_user(&self, id: i64) -> Result<User> {
        if id > 0 && id <= 3 {
            Ok(User {
                id,
                name: format!("User-{id}"),
                email: format!("user{id}@example.com"),
            })
        } else {
            Err(Error::Http {
                status: StatusCode::NOT_FOUND,
                message: format!("User {id} not found"),
            })
        }
    }
}

// ─── 启动入口 ─────────────────────────────────────────────

#[webr::main]
async fn main(app: &mut AppBuilder) -> Result<()> {
    app.unified_response();
    // Cache 自动从 [cache] 配置初始化，无需手动 provide
    Ok(())
}
