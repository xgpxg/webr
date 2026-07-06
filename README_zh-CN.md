<p align="center">
  <img src="docs/public/logo.png" alt="WebR Logo" width="150">
</p>

# WebR

[English](README.md)

WebR 将 Spring Boot 的开发体验带入 Rust 生态 ——
宏驱动的控制器、自动依赖注入、配置管理、中间件系统，构建于 [Axum](https://github.com/tokio-rs/axum) 之上。

## 功能特性

- **宏驱动控制器** — `#[controller]` 配合 `#[get]`、`#[post]` 等路由宏，零样板定义接口。
- **依赖注入** — `#[component]` 声明组件，`Inject<T>` 自动注入，启动时拓扑排序解析依赖。
- **配置系统** — 多文件 TOML + profile 切换 + 环境变量覆盖，`#[config]` 绑定即注入。
- **中间件** — 全局 / 路径范围中间件，实现 `Middleware` trait 即可。内置 CORS、日志、Panic 恢复、统一响应。
- **认证鉴权** — `AuthMiddleware` + `Authenticator` trait，支持路径级守卫。
- **请求校验** — `Json`、`Query`、`Form` 提取器自动校验，基于 `validator` crate。
- **文件上传下载** — `Multipart` 提取器 + `FileResponse` 支持字节 / 路径 / 内联响应。
- **SSE 推送** — `SseResponse` + `SseEvent` 实现服务端推送。
- **声明式错误** — `#[derive(HttpError)]` 定义业务错误，自动映射 HTTP 状态码。
- **数据库** — 连接池、`#[sql]` 动态查询、`#[tx]` 事务管理（Feature-gated，支持 MySQL / PostgreSQL / SQLite）。
- **缓存** — 内存 / Sled / Redis 多后端，Feature-gated 按需启用。

## 快速开始

### 1. 添加依赖

```toml
[dependencies]
webr = "0.1"
serde = { version = "1", features = ["derive"] }
```

### 2. 编写配置

`config/application.toml`：

```toml
[server]
port = 8080

[app]
name = "my-app"
greeting = "Hello from WebR!"
```

### 3. 编写代码

`src/main.rs`：

```rust
use webr::prelude::*;
use webr::{Inject, Error};

// 配置绑定 —— 结构体字段自动从 [app] 节加载
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}

// 业务组件 —— 自动注册到 DI 容器
#[component]
pub struct UserService;

impl UserService {
    pub async fn find_all(&self) -> Vec<String> {
        vec!["Alice".into(), "Bob".into()]
    }
}

// 控制器 —— 通过 Inject<T> 声明依赖
#[controller]
pub struct HelloController {
    app_config: Inject<AppConfig>,
    user_service: Inject<UserService>,
}

#[controller]
impl HelloController {
    #[get("/")]
    async fn index(&self) -> String {
        self.app_config.greeting.clone()
    }

    #[get("/users")]
    async fn users(&self) -> webr::Json<Vec<String>> {
        webr::Json(self.user_service.find_all().await)
    }
}

// 启动入口
#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), Error> {
    app.unified_response(); // 启用统一响应包装（可选）
    Ok(())
}
```

运行 `cargo run`，访问 `http://localhost:8080/` 即可看到效果。

## 核心概念

### 依赖注入

使用 `#[component]` 或 `#[controller]` 标注的类型会自动注册到 DI 容器。框架在启动时通过拓扑排序解析所有依赖关系，无需手动装配。

在字段中使用 `Inject<T>` 声明依赖，其行为类似 `Arc<T>`，支持透明 `Deref`：

```rust
#[controller]
pub struct UserController {
    user_service: Inject<UserService>,  // 自动解析并注入
}
```

`#[config]` 标注的结构体同样可注入：

```rust
#[controller]
pub struct MyController {
    config: Inject<AppConfig>,  // 配置对象直接注入
}
```

### 配置管理

配置按以下优先级加载，后者覆盖前者：

| 优先级 | 来源                                  | 说明                           |
|:---:|-------------------------------------|------------------------------|
|  1  | `config/application.toml`           | 基础配置                         |
|  2  | `config/application-{profile}.toml` | Profile 配置，profile 默认为 `dev` |
|  3  | `WEBR_` 前缀环境变量                      | 如 `WEBR_SERVER_PORT=9090`    |

通过 `WEBR_PROFILE` 环境变量切换 profile（如 `prod`、`test`）。

使用 `#[config(prefix = "section")]` 将结构体绑定到配置节，绑定后即可通过 `Inject<T>` 注入：

```rust
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}
```

### 路由与控制器

路由通过控制器宏定义，支持所有标准 HTTP 方法：

```rust
#[controller]
pub struct ItemController;

#[controller]
impl ItemController {
    #[get("/items")]        // GET    /items
    async fn list(&self) -> webr::Json<Vec<Item>> { /* ... */ }

    #[get("/items/:id")]    // GET    /items/:id
    async fn get(&self, webr::Path(id): webr::Path<i64>) -> webr::Json<Item> { /* ... */ }

    #[post("/items")]       // POST   /items
    async fn create(&self, webr::Json(dto): webr::Json<CreateDto>) -> StatusCode { /* ... */ }

    #[put("/items/:id")]    // PUT    /items/:id
    async fn update(&self, /* ... */) -> webr::Json<Item> { /* ... */ }

    #[delete("/items/:id")] // DELETE /items/:id
    async fn delete(&self, /* ... */) -> StatusCode { /* ... */ }
}
```

### 中间件

#### 注册方式

```rust
#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), Error> {
    // 全局中间件
    app.middleware(PanicRecovery);
    app.middleware(LoggerMiddleware);
    app.middleware(CorsMiddleware::new().allow_origin("*"));

    // 路径范围中间件 —— 仅匹配指定路径
    app.middleware_for("/api/**", RequireAuth);

    // 路径排除中间件 —— 匹配所有 except 指定路径以外的请求
    app.middleware_except("/health", LoggerMiddleware);

    Ok(())
}
```

#### 内置中间件

| 中间件                    | 说明                                                  |
|------------------------|-----------------------------------------------------|
| `LoggerMiddleware`     | 请求日志：方法、路径、状态码、耗时                                   |
| `CorsMiddleware`       | Builder 模式配置 CORS 跨域策略                              |
| `PanicRecovery`        | 捕获 handler panic，返回 500 而非进程崩溃                      |
| `UnifiedResponse`      | 将 2xx JSON 响应包装为 `{"code", "message", "data"}` 标准格式 |
| `AuthMiddleware`       | 认证鉴权，配合 `Authenticator` trait 使用                    |
| `CachedBodyMiddleware` | 缓存请求体，供多次读取                                         |

#### 自定义中间件

实现 `Middleware` trait 即可：

```rust
pub struct MyMiddleware;

#[async_trait]
impl Middleware for MyMiddleware {
    async fn handle(&self, request: Request, next: Next) -> Response {
        // 前置处理
        let response = next.run(request).await;
        // 后置处理
        response
    }
}
```

### 认证鉴权

实现 `Authenticator` trait 定义认证逻辑，通过 `AuthMiddleware` 注册：

```rust
#[component]
pub struct MyAuthenticator;

#[async_trait]
impl Authenticator for MyAuthenticator {
    async fn authenticate(&self, request: &Request) -> Result<UserInfo, AuthError> {
        // 从 Header / Cookie / Token 中提取并验证用户身份
    }
}
```

在控制器中通过 `CurrentUser<T>` 提取当前用户：

```rust
#[controller]
impl ApiController {
    #[get("/api/profile")]
    async fn profile(&self, user: webr::CurrentUser<UserInfo>) -> webr::Json<UserInfo> {
        webr::Json(user.0)
    }
}
```

### 请求校验

在 DTO 上 derive `Validate`，提取器自动执行校验：

```rust
#[derive(Deserialize, Validate)]
pub struct CreateUserDto {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(range(min = 18, max = 150))]
    pub age: u8,
}

#[controller]
struct UserController;

#[controller]
impl UserController {
    #[post("/users")]
    async fn create(&self, webr::Json(dto): webr::Json<CreateUserDto>) -> webr::Json<User> {
        // dto 已通过校验，可直接使用
        todo!()
    }
}
```

校验失败时自动返回 `422 Unprocessable Entity` 及字段级错误详情。

### 错误处理

使用 `#[derive(HttpError)]` 声明式定义业务错误类型：

```rust
#[derive(Debug, webr::HttpError)]
pub enum UserError {
    #[error(status = 404, message = "User not found")]
    NotFound(i64),
    #[error(status = 409, message = "Email already exists")]
    DuplicateEmail(String),
}
```

在控制器中使用 `WebrResult<T>` 作为返回类型，错误通过 `?` 自动转为 HTTP 响应：

```rust
async fn get_user(&self, id: i64) -> WebrResult<webr::Json<User>> {
    let user = self.service.find(id).await.ok_or(UserError::NotFound(id))?;
    Ok(webr::Json(user))
}
```

### 文件上传与下载

```rust
// 上传
#[post("/upload")]
async fn upload(&self, mut multipart: webr::Multipart) -> webr::Json<Vec<String>> {
    let mut filenames = Vec::new();
    while let Ok(field) = multipart.next_field().await {
        if let Some(name) = field.file_name() {
            filenames.push(name.to_string());
        }
    }
    webr::Json(filenames)
}

// 下载
#[get("/download/:filename")]
async fn download(&self, webr::Path(filename): webr::Path<String>) -> webr::FileResponse {
    webr::FileResponse::from_path(format!("./uploads/{}", filename))
}
```

### SSE 服务端推送

```rust
use webr::{SseResponse, SseEvent};

#[controller]
impl EventController {
    #[get("/events")]
    async fn stream(&self) -> SseResponse {
        SseResponse::new(|tx| async move {
            for i in 0..10 {
                let event = SseEvent::default().data(format!("message {}", i));
                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
    }
}
```

## 可选模块

以下模块通过 Cargo Feature 按需启用：

### 数据库

启用对应数据库的 feature：

```toml
[dependencies]
webr = { version = "0.1", features = ["mysql"] }    # 或 "postgres"、"sqlite"
```

支持 `#[sql]` 动态查询、`#[tx]` 事务管理、`#[entity]` 实体定义。详见 [`orm` 示例](examples/orm)。

### 缓存

启用对应后端的 feature：

```toml
[dependencies]
webr = { version = "0.1", features = ["cache-memory"] }  # 或 "cache-sled"、"cache-redis"
```

支持内存、Sled、Redis 三种后端。详见 [`cache` 示例](examples/cache)。

## 示例

| 示例                                    | 说明                               |
|---------------------------------------|----------------------------------|
| [`hello-world`](examples/hello-world) | 控制器、依赖注入、配置绑定、统一响应               |
| [`middleware`](examples/middleware)   | 自定义认证中间件、路径范围路由、CORS             |
| [`validation`](examples/validation)   | JSON / Query / Form 的 DTO 校验     |
| [`file-upload`](examples/file-upload) | 多文件上传、文件下载、内联预览                  |
| [`sse`](examples/sse)                 | Server-Sent Events 服务端推送         |
| [`orm`](examples/orm)                 | 实体 CRUD、`#[sql]` 动态查询、`#[tx]` 事务 |
| [`cache`](examples/cache)             | 缓存模块使用                           |

运行示例：

```bash
cd examples/hello-world
cargo run
```

## 项目结构

```
webr/
├── crates/
│   ├── webr-core/        # 核心框架：DI、配置、中间件、路由、提取器、响应
│   ├── webr-db/          # 数据库：连接池、事务、ORM 支持
│   ├── webr-cache/       # 缓存：内存 / Sled / Redis 多后端
│   ├── webr-macros/      # 过程宏：controller、component、config、entity、sql、tx、Validate
│   └── webr-middleware/  # 中间件：认证鉴权、请求体缓存
├── examples/             # 示例应用
└── src/lib.rs            # Umbrella crate，统一导出所有公共 API
```

## 技术栈

| 组件      | 依赖                                              |
|---------|-------------------------------------------------|
| HTTP 框架 | [Axum 0.8](https://crates.io/crates/axum)       |
| 异步运行时   | [Tokio](https://crates.io/crates/tokio)         |
| 序列化     | [Serde](https://crates.io/crates/serde)         |
| 参数校验    | [validator](https://crates.io/crates/validator) |
| 配置      | [toml](https://crates.io/crates/toml)           |
| 日志      | [tracing](https://crates.io/crates/tracing)     |
| 组件注册    | [inventory](https://crates.io/crates/inventory) |

## 许可证

MIT
