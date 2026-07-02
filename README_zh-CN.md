# WebR

**一个受 Spring Boot 启发的 Rust Web 框架。**

[English](README.md)

WebR 将 Spring Boot 熟悉的开发体验带入 Rust 生态 —— 注解驱动的控制器、自动依赖注入、配置管理、内置中间件系统，一切构建于 [Axum 0.8](https://github.com/tokio-rs/axum) 之上。

> **当前状态：** `webr-core` 已可用，其他模块（`webr-db`、`webr-macros` ORM 功能）正在开发中。

## 功能特性

- **注解驱动控制器** — `#[controller]` + `#[get]`、`#[post]` 等注解，零样板代码定义路由。
- **依赖注入** — `Inject<T>` 智能指针，启动时自动拓扑排序解析依赖，无需手动装配。
- **配置系统** — 多文件 TOML 配置，支持 profile（`application-{profile}.toml`）和环境变量覆盖（`WEBR_` 前缀）。
- **中间件系统** — 全局和路径范围中间件，简单的 trait 即可实现。内置：CORS、日志、Panic 恢复、统一响应。
- **请求校验** — `Json`、`Query`、`Form` 提取器自动校验，基于 `validator` crate。
- **统一响应** — 一行 `app.unified_response()` 将所有 2xx JSON 包装为 `{"code", "message", "data"}` 格式。
- **文件上传下载** — `Multipart` 提取器和 `FileResponse` 支持字节/路径/内联响应。
- **自定义错误处理** — `#[derive(HttpError)]` 声明式定义 HTTP 错误类型。

## 快速开始

### `Cargo.toml`

```toml
[dependencies]
webr = "0.1"
serde = { version = "1", features = ["derive"] }
```

### `config/application.toml`

```toml
[server]
port = 8080

[app]
name = "my-app"
greeting = "Hello from WebR!"
```

### `src/main.rs`

```rust
use webr::prelude::*;
use webr::{Inject, WebrError};

// ── 配置 ───────────────────────────────────────────────

#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}

// ── Service 层 ─────────────────────────────────────────

#[component]
pub struct UserService;

impl UserService {
    pub async fn find_all(&self) -> Vec<String> {
        vec!["Alice".into(), "Bob".into()]
    }
}

// ── Controller 层 ──────────────────────────────────────

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

// ── 启动入口 ───────────────────────────────────────────

#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), WebrError> {
    app.unified_response();
    Ok(())
}
```

## 核心概念

### 依赖注入

使用 `#[component]` 或 `#[controller]` 标注的组件通过 `inventory` crate 自动注册。框架在启动时通过拓扑排序解析依赖。

使用 `Inject<T>` 声明依赖 —— 其行为类似 `Arc<T>` 并支持透明 `Deref`：

```rust
#[controller]
pub struct UserController {
    user_service: Inject<UserService>,  // 自动解析
}
```

### 配置管理

配置从 TOML 文件加载，优先级如下（后者覆盖前者）：

1. `config/application.toml`
2. `config/application-{profile}.toml`（profile 默认为 `dev`，通过 `WEBR_PROFILE` 设置）
3. `WEBR_` 前缀的环境变量（如 `WEBR_SERVER_PORT=9090`）

使用 `#[config(prefix = "section")]` 将结构体绑定到配置节并使其可注入：

```rust
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}
// 自动从 [app] 配置节加载，可通过 Inject<AppConfig> 注入
```

### 中间件

实现 `Middleware` trait 并全局注册或针对特定路径：

```rust
#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), WebrError> {
    // 全局中间件
    app.middleware(PanicRecovery);
    app.middleware(LoggerMiddleware);
    app.middleware(CorsMiddleware::new().allow_origin("*"));

    // 路径范围中间件
    app.middleware_for("/api/**", RequireAuth);
    app.middleware_except("/health", LoggerMiddleware);

    // 统一响应包装
    app.unified_response();

    Ok(())
}
```

**内置中间件：**

| 中间件 | 描述 |
|---|---|
| `LoggerMiddleware` | 请求日志，记录方法、路径、状态码、耗时 |
| `CorsMiddleware` | Builder 模式的 CORS 跨域配置 |
| `PanicRecovery` | 捕获 handler panic，返回 500 而非进程崩溃 |
| `UnifiedResponse` | 将 2xx JSON 包装为 `{"code", "message", "data"}` 标准格式 |

### 请求校验

在 DTO 上 derive `Validate` —— 提取器（`Json`、`Query`、`Form`）自动校验：

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

#[post("/users")]
async fn create_user(&self, webr::Json(dto): webr::Json<CreateUserDto>) -> webr::Json<User> {
    // dto 已经校验通过
    // ...
}
```

校验失败自动返回 `422` 及字段级错误详情。

### 错误处理

使用 `#[derive(HttpError)]` 声明式定义业务错误：

```rust
#[derive(Debug, webr::HttpError)]
pub enum UserError {
    #[error(status = 404, message = "User not found")]
    NotFound(i64),
    #[error(status = 409, message = "Email already exists")]
    DuplicateEmail(String),
}
```

使用 `WebrResult<T>` 作为 handler 返回类型 —— 错误通过 `?` 自动转为 HTTP 响应。

## 示例

参见 [`examples/`](examples/) 目录：

| 示例 | 描述 |
|---|---|
| [`hello-world`](examples/hello-world) | 基础控制器、依赖注入、配置、统一响应 |
| [`middleware`](examples/middleware) | 自定义鉴权中间件、路径范围路由、CORS |
| [`validation`](examples/validation) | JSON 请求体、查询参数、表单数据的 DTO 校验 |
| [`file-upload`](examples/file-upload) | 多文件上传、文件下载、内联预览 |

运行示例：

```bash
cd examples/hello-world
cargo run
```

## 项目结构

```
webr/
├── crates/
│   ├── webr-core/      # 核心框架（DI、配置、中间件、路由、提取器）
│   ├── webr-db/        # 数据库模块（开发中）
│   └── webr-macros/    # 过程宏（开发中）
├── examples/           # 示例应用
└── src/lib.rs          # 统一导出
```

## 技术栈

| 组件 | Crate |
|---|---|
| HTTP 框架 | [Axum 0.8](https://crates.io/crates/axum) |
| 异步运行时 | [Tokio](https://crates.io/crates/tokio) |
| 序列化 | [Serde](https://crates.io/crates/serde) |
| 参数校验 | [validator](https://crates.io/crates/validator) |
| 配置 | [toml](https://crates.io/crates/toml) |
| 日志 | [tracing](https://crates.io/crates/tracing) |
| 自动注册 | [inventory](https://crates.io/crates/inventory) |

## 许可证

MIT
