# 中间件

WebR 的中间件系统支持全局、路径范围和排除三种注册方式。

## 定义中间件

实现 `Middleware` trait：

```rust
use webr::async_trait::async_trait;
use webr::middleware::{Middleware, Next};
use axum::extract::Request;
use axum::response::Response;

pub struct MyMiddleware;

#[async_trait]
impl Middleware for MyMiddleware {
    async fn handle(&self, request: Request, next: Next) -> Response {
        // 前置处理
        println!("before handler");

        let response = next.run(request).await;

        // 后置处理
        println!("after handler");
        response
    }
}
```

## 注册中间件

### 全局中间件

对所有路由生效：

```rust
#[webr::main]
async fn main(app: &mut AppBuilder) -> Result<(), Error> {
    app.middleware(LoggerMiddleware);
    app.middleware(PanicRecovery);
    app.middleware(CorsMiddleware::new().allow_origin("*"));
    // 启用统一响应包装（便捷方法）
    app.unified_response();
    Ok(())
}
```

### 路径范围中间件

仅对匹配指定路径模式的路由生效：

```rust
// 前缀匹配：/api/** 下的所有路径
app.middleware_for("/api/**", RequireAuth);

// 精确匹配：仅 /admin 路径
app.middleware_for("/admin", AdminOnlyMiddleware);
```

### 排除模式中间件

对除了匹配路径之外的所有路由生效：

```rust
// 除了 /health 路径，其他所有请求都走 LoggerMiddleware
app.middleware_except("/health", LoggerMiddleware);
```

### 执行顺序

中间件按注册顺序依次执行，构成洋葱模型：

```
Middleware1 → Middleware2 → ... → Handler → ... → Middleware2 → Middleware1
```

## 内置中间件

### LoggerMiddleware

请求日志记录：方法、路径、状态码、耗时。

```rust
app.middleware(LoggerMiddleware);
```

日志输出：

```
-> GET /api/users
<- GET /api/users 200 OK (1.23ms)
```

### CorsMiddleware

CORS 跨域配置，Builder 模式链式调用。

```rust
// 默认配置：允许所有来源
app.middleware(CorsMiddleware::new());

// 自定义配置
app.middleware(
    CorsMiddleware::new()
        .allow_origin("https://example.com")
        .allow_methods(["GET", "POST"])
        .allow_headers(["Content-Type", "Authorization"])
        .allow_credentials(true)
        .max_age(3600),
);
```

默认值：

| 配置项 | 默认值 |
|--------|--------|
| allow_origin | `*` |
| allow_methods | `GET,POST,PUT,DELETE,PATCH,OPTIONS` |
| allow_headers | `Content-Type,Authorization` |
| allow_credentials | `false` |

注意：`allow_credentials(true)` 和 `allow_origin("*")` 互斥，同时设置会 panic。

### PanicRecovery

捕获 handler 中的 panic，返回 500 错误阻止进程崩溃。

```rust
app.middleware(PanicRecovery);
```

### UnifiedResponse

将 2xx JSON 响应包装为统一格式 `{"code": 200, "message": "success", "data": ...}`。

```rust
app.unified_response();  // 推荐
// 等价于: app.middleware(UnifiedResponse);
```

规则：

| 响应类型 | 处理方式 |
|----------|----------|
| 2xx + JSON | 包装为标准格式 |
| 非 2xx | 原样透传 |
| 非 JSON | 原样透传 |

### AuthMiddleware

认证中间件，需配合 `Authenticator` trait 使用：

```rust
// 全局认证
app.middleware(AuthMiddleware::new(JwtAuth));

// 排除公开路径
app.middleware_except("/login", AuthMiddleware::new(JwtAuth));
```

详见 [认证机制](#认证与鉴权) 章节。

### CachedBodyMiddleware

缓存请求体到内存，解决 body 只能消费一次的问题。需在读取 body 的中间件之前注册：

```rust
app.middleware(CachedBodyMiddleware);
app.middleware(AuthMiddleware::new(WebhookAuth)); // 需要读取 body
```

## 认证与鉴权

### Authenticator

实现 `Authenticator` trait 定义认证逻辑：

```rust
use webr::Authenticator;

struct JwtAuth;

#[async_trait]
impl Authenticator for JwtAuth {
    type Identity = UserInfo;

    async fn authenticate(
        &self,
        headers: &HeaderMap,
        body: Option<&Bytes>,
    ) -> Result<UserInfo, AuthError> {
        let token = headers.get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::new("Missing token"))?;
        // 验证 token，返回用户身份
        decode_jwt(token).map_err(|e| AuthError::new(e.to_string()))
    }
}
```

### CurrentUser

在控制器中提取已认证的用户身份：

```rust
#[controller]
impl ApiController {
    #[get("/profile")]
    async fn profile(&self, CurrentUser(user): CurrentUser<UserInfo>) -> Json<UserInfo> {
        Json(user)
    }
}
```

### Guard (鉴权守卫)

实现 `Guard` trait 做细粒度权限检查：

```rust
struct AdminGuard;

#[async_trait]
impl Guard for AdminGuard {
    async fn check(&self, req: &Request) -> Result<(), Error> {
        let user = req.extensions().get::<UserInfo>()
            .ok_or_else(|| Error::Http {
                status: StatusCode::UNAUTHORIZED,
                message: "Not authenticated".into(),
            })?;
        if user.role != "admin" {
            return Err(Error::Http {
                status: StatusCode::FORBIDDEN,
                message: "Admin access required".into(),
            });
        }
        Ok(())
    }
}

// 注册
app.middleware_for("/admin/**", GuardMiddleware::new(AdminGuard));
```
