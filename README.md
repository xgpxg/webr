# WebR

**A Spring Boot-inspired web framework for Rust.**

[中文文档](README_zh-CN.md)

WebR brings the familiar developer experience of Spring Boot to the Rust ecosystem — annotation-driven controllers, automatic dependency injection, configuration management, and a built-in middleware system, all built on top of [Axum 0.8](https://github.com/tokio-rs/axum).

> **Status:** `webr-core` is currently available. Other modules (`webr-db`, `webr-macros` ORM features) are under active development.

## Features

- **Annotation-Driven Controllers** — `#[controller]` + `#[get]`, `#[post]` etc. for zero-boilerplate route definitions.
- **Dependency Injection** — `Inject<T>` smart pointer with automatic topological resolution. No manual wiring.
- **Configuration System** — Multi-file TOML with profile support (`application-{profile}.toml`) and environment variable overrides (`WEBR_` prefix).
- **Middleware System** — Global and path-scoped middleware with a simple trait. Built-in: CORS, Logger, Panic Recovery, Unified Response.
- **Request Validation** — Automatic validation on `Json`, `Query`, `Form` extractors via the `validator` crate.
- **Unified Response** — One-line `app.unified_response()` wraps all 2xx JSON into `{"code", "message", "data"}`.
- **File Upload & Download** — `Multipart` extractor and `FileResponse` for byte/path/inline responses.
- **Custom Error Handling** — `#[derive(HttpError)]` for declarative HTTP error types.

## Quick Start

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

// ── Config ─────────────────────────────────────────────

#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}

// ── Service ────────────────────────────────────────────

#[component]
pub struct UserService;

impl UserService {
    pub async fn find_all(&self) -> Vec<String> {
        vec!["Alice".into(), "Bob".into()]
    }
}

// ── Controller ─────────────────────────────────────────

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

// ── Entry ──────────────────────────────────────────────

#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), WebrError> {
    app.unified_response();
    Ok(())
}
```

## Core Concepts

### Dependency Injection

Components annotated with `#[component]` or `#[controller]` are auto-registered via the `inventory` crate. The framework resolves dependencies at startup using topological sorting.

Use `Inject<T>` to declare dependencies — it behaves like `Arc<T>` with transparent `Deref`:

```rust
#[controller]
pub struct UserController {
    user_service: Inject<UserService>,  // auto-resolved
}
```

### Configuration

Configuration is loaded from TOML files with the following priority (later overrides earlier):

1. `config/application.toml`
2. `config/application-{profile}.toml` (profile defaults to `dev`, set via `WEBR_PROFILE`)
3. Environment variables with `WEBR_` prefix (e.g., `WEBR_SERVER_PORT=9090`)

Use `#[config(prefix = "section")]` to bind a struct to a config section and make it injectable:

```rust
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}
// Automatically loaded from [app] section, injectable via Inject<AppConfig>
```

### Middleware

Implement the `Middleware` trait and register globally or on specific paths:

```rust
#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), WebrError> {
    // Global middleware
    app.middleware(PanicRecovery);
    app.middleware(LoggerMiddleware);
    app.middleware(CorsMiddleware::new().allow_origin("*"));

    // Path-scoped middleware
    app.middleware_for("/api/**", RequireAuth);
    app.middleware_except("/health", LoggerMiddleware);

    // Unified response wrapper
    app.unified_response();

    Ok(())
}
```

**Built-in middleware:**

| Middleware | Description |
|---|---|
| `LoggerMiddleware` | Request logging with method, path, status, and duration |
| `CorsMiddleware` | Builder-pattern CORS configuration |
| `PanicRecovery` | Catches handler panics, returns 500 instead of crashing |
| `UnifiedResponse` | Wraps 2xx JSON as `{"code", "message", "data"}` |

### Request Validation

Derive `Validate` on DTOs — extractors (`Json`, `Query`, `Form`) validate automatically:

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
    // dto is already validated
    // ...
}
```

Validation failures return `422` with per-field error details automatically.

### Error Handling

Use `#[derive(HttpError)]` for declarative business errors:

```rust
#[derive(Debug, webr::HttpError)]
pub enum UserError {
    #[error(status = 404, message = "User not found")]
    NotFound(i64),
    #[error(status = 409, message = "Email already exists")]
    DuplicateEmail(String),
}
```

Use `WebrResult<T>` as handler return type — errors convert to HTTP responses automatically via `?`.

## Examples

See the [`examples/`](examples/) directory:

| Example | Description |
|---|---|
| [`hello-world`](examples/hello-world) | Basic controller, DI, config, unified response |
| [`middleware`](examples/middleware) | Custom auth middleware, scoped routing, CORS |
| [`validation`](examples/validation) | DTO validation on JSON body, query params, form data |
| [`file-upload`](examples/file-upload) | Multipart upload, file download, inline preview |

Run an example:

```bash
cd examples/hello-world
cargo run
```

## Project Structure

```
webr/
├── crates/
│   ├── webr-core/      # Core framework (DI, config, middleware, routing, extractors)
│   ├── webr-db/        # Database module (under development)
│   └── webr-macros/    # Procedural macros (under development)
├── examples/           # Example applications
└── src/lib.rs          # Re-exports
```

## Tech Stack

| Component | Crate |
|---|---|
| HTTP Framework | [Axum 0.8](https://crates.io/crates/axum) |
| Async Runtime | [Tokio](https://crates.io/crates/tokio) |
| Serialization | [Serde](https://crates.io/crates/serde) |
| Validation | [validator](https://crates.io/crates/validator) |
| Configuration | [toml](https://crates.io/crates/toml) |
| Logging | [tracing](https://crates.io/crates/tracing) |
| Auto-registration | [inventory](https://crates.io/crates/inventory) |

## License

MIT
