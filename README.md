<p align="center">
  <img src="docs/public/logo.png" alt="WebR Logo" width="150">
</p>

# WebR

**A Spring Boot-inspired web framework for Rust.**

[中文文档](README_zh-CN.md)

WebR brings the developer experience of Spring Boot to the Rust ecosystem — macro-driven controllers, automatic
dependency injection, configuration management, and a built-in middleware system, all built on top
of [Axum](https://github.com/tokio-rs/axum).

## Features

- **Macro-Driven Controllers** — `#[controller]` with `#[get]`, `#[post]`, etc. for zero-boilerplate route definitions.
- **Dependency Injection** — `#[component]` declares components, `Inject<T>` auto-injects. Dependencies resolved via
  topological sort at startup.
- **Configuration System** — Multi-file TOML + profile switching + environment variable overrides. `#[config]` binds and
  injects.
- **Middleware** — Global / path-scoped middleware via a simple trait. Built-in: CORS, Logger, Panic Recovery, Unified
  Response.
- **Authentication** — `AuthMiddleware` + `Authenticator` trait for path-level guards.
- **Request Validation** — `Json`, `Query`, `Form` extractors validate automatically via the `validator` crate.
- **File Upload & Download** — `Multipart` extractor and `FileResponse` for byte / path / inline responses.
- **SSE** — `SseResponse` + `SseEvent` for server-sent events.
- **Declarative Errors** — `#[derive(HttpError)]` maps business errors to HTTP status codes.
- **Database** — Connection pool, `#[sql]` dynamic queries, `#[tx]` transaction management (feature-gated: MySQL /
  PostgreSQL / SQLite).
- **Cache** — Memory / Sled / Redis backends, feature-gated.

## Quick Start

### 1. Add Dependencies

```toml
[dependencies]
webr = "0.1"
serde = { version = "1", features = ["derive"] }
```

### 2. Write Configuration

`config/application.toml`:

```toml
[server]
port = 8080

[app]
name = "my-app"
greeting = "Hello from WebR!"
```

### 3. Write Code

`src/main.rs`:

```rust
use webr::prelude::*;
use webr::{Inject, Error};

// Config binding — fields auto-loaded from [app] section
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}

// Business component — auto-registered in the DI container
#[component]
pub struct UserService;

impl UserService {
    pub async fn find_all(&self) -> Vec<String> {
        vec!["Alice".into(), "Bob".into()]
    }
}

// Controller — declare dependencies via Inject<T>
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

// Entry point
#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), Error> {
    app.unified_response(); // Enable unified response wrapping (optional)
    Ok(())
}
```

Run `cargo run` and visit `http://localhost:8080/`.

## Core Concepts

### Dependency Injection

Types annotated with `#[component]` or `#[controller]` are automatically registered in the DI container. The framework
resolves all dependencies at startup using topological sorting — no manual wiring required.

Use `Inject<T>` in fields to declare dependencies. It behaves like `Arc<T>` with transparent `Deref`:

```rust
#[controller]
pub struct UserController {
    user_service: Inject<UserService>,  // auto-resolved and injected
}
```

`#[config]`-annotated structs are also injectable:

```rust
#[controller]
pub struct MyController {
    config: Inject<AppConfig>,  // config object injected directly
}
```

### Configuration

Configuration is loaded in the following priority order (later overrides earlier):

| Priority | Source                                 | Description                               |
|:--------:|----------------------------------------|-------------------------------------------|
|    1     | `config/application.toml`              | Base configuration                        |
|    2     | `config/application-{profile}.toml`    | Profile config, profile defaults to `dev` |
|    3     | `WEBR_` prefixed environment variables | e.g. `WEBR_SERVER_PORT=9090`              |

Set the `WEBR_PROFILE` environment variable to switch profiles (e.g. `prod`, `test`).

Use `#[config(prefix = "section")]` to bind a struct to a config section, making it injectable:

```rust
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}
```

### Routing & Controllers

Routes are defined via controller macros, supporting all standard HTTP methods:

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

### Middleware

#### Registration

```rust
#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), Error> {
    // Global middleware
    app.middleware(PanicRecovery);
    app.middleware(LoggerMiddleware);
    app.middleware(CorsMiddleware::new().allow_origin("*"));

    // Path-scoped middleware — only matches specified paths
    app.middleware_for("/api/**", RequireAuth);

    // Path-excluded middleware — matches all requests except specified paths
    app.middleware_except("/health", LoggerMiddleware);

    Ok(())
}
```

#### Built-in Middleware

| Middleware             | Description                                                        |
|------------------------|--------------------------------------------------------------------|
| `LoggerMiddleware`     | Request logging: method, path, status code, duration               |
| `CorsMiddleware`       | Builder-pattern CORS configuration                                 |
| `PanicRecovery`        | Catches handler panics, returns 500 instead of crashing            |
| `UnifiedResponse`      | Wraps 2xx JSON responses into `{"code", "message", "data"}` format |
| `AuthMiddleware`       | Authentication & authorization via the `Authenticator` trait       |
| `CachedBodyMiddleware` | Caches request body for multiple reads                             |

#### Custom Middleware

Implement the `Middleware` trait:

```rust
pub struct MyMiddleware;

#[async_trait]
impl Middleware for MyMiddleware {
    async fn handle(&self, request: Request, next: Next) -> Response {
        // Pre-processing
        let response = next.run(request).await;
        // Post-processing
        response
    }
}
```

### Authentication

Implement the `Authenticator` trait to define authentication logic, then register via `AuthMiddleware`:

```rust
#[component]
pub struct MyAuthenticator;

#[async_trait]
impl Authenticator for MyAuthenticator {
    async fn authenticate(&self, request: &Request) -> Result<UserInfo, AuthError> {
        // Extract and verify user identity from Header / Cookie / Token
    }
}
```

Extract the current user in controllers via `CurrentUser<T>`:

```rust
#[controller]
impl ApiController {
    #[get("/api/profile")]
    async fn profile(&self, user: webr::CurrentUser<UserInfo>) -> webr::Json<UserInfo> {
        webr::Json(user.0)
    }
}
```

### Request Validation

Derive `Validate` on DTOs — extractors validate automatically:

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
        // dto is already validated
        todo!()
    }
}
```

Validation failures automatically return `422 Unprocessable Entity` with per-field error details.

### Error Handling

Use `#[derive(HttpError)]` to declaratively define business error types:

```rust
#[derive(Debug, webr::HttpError)]
pub enum UserError {
    #[error(status = 404, message = "User not found")]
    NotFound(i64),
    #[error(status = 409, message = "Email already exists")]
    DuplicateEmail(String),
}
```

Use `WebrResult<T>` as the handler return type — errors convert to HTTP responses automatically via `?`:

```rust
async fn get_user(&self, id: i64) -> WebrResult<webr::Json<User>> {
    let user = self.service.find(id).await.ok_or(UserError::NotFound(id))?;
    Ok(webr::Json(user))
}
```

### File Upload & Download

```rust
// Upload
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

// Download
#[get("/download/:filename")]
async fn download(&self, webr::Path(filename): webr::Path<String>) -> webr::FileResponse {
    webr::FileResponse::from_path(format!("./uploads/{}", filename))
}
```

### SSE (Server-Sent Events)

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

## Optional Modules

The following modules are enabled via Cargo features on demand:

### Database

Enable the feature for your database:

```toml
[dependencies]
webr = { version = "0.1", features = ["mysql"] }    # or "postgres", "sqlite"
```

Supports `#[sql]` dynamic queries, `#[tx]` transaction management, `#[entity]` entity definitions. See the [
`orm` example](examples/orm).

### Cache

Enable the feature for your backend:

```toml
[dependencies]
webr = { version = "0.1", features = ["cache-memory"] }  # or "cache-sled", "cache-redis"
```

Supports Memory, Sled, and Redis backends. See the [`cache` example](examples/cache).

## Examples

| Example                               | Description                                                 |
|---------------------------------------|-------------------------------------------------------------|
| [`hello-world`](examples/hello-world) | Controllers, DI, config binding, unified response           |
| [`middleware`](examples/middleware)   | Custom auth middleware, path-scoped routing, CORS           |
| [`validation`](examples/validation)   | DTO validation on JSON / Query / Form                       |
| [`file-upload`](examples/file-upload) | Multi-file upload, file download, inline preview            |
| [`sse`](examples/sse)                 | Server-Sent Events streaming                                |
| [`orm`](examples/orm)                 | Entity CRUD, `#[sql]` dynamic queries, `#[tx]` transactions |
| [`cache`](examples/cache)             | Cache module usage                                          |

Run an example:

```bash
cd examples/hello-world
cargo run
```

## Project Structure

```
webr/
├── crates/
│   ├── webr-core/        # Core framework: DI, config, middleware, routing, extractors, response
│   ├── webr-db/          # Database: connection pool, transactions, ORM support
│   ├── webr-cache/       # Cache: Memory / Sled / Redis backends
│   ├── webr-macros/      # Procedural macros: controller, component, config, entity, sql, tx, Validate
│   └── webr-middleware/  # Middleware: authentication, request body caching
├── examples/             # Example applications
└── src/lib.rs            # Umbrella crate, unified re-export of all public APIs
```

## Tech Stack

| Component         | Dependency                                      |
|-------------------|-------------------------------------------------|
| HTTP Framework    | [Axum 0.8](https://crates.io/crates/axum)       |
| Async Runtime     | [Tokio](https://crates.io/crates/tokio)         |
| Serialization     | [Serde](https://crates.io/crates/serde)         |
| Validation        | [validator](https://crates.io/crates/validator) |
| Configuration     | [toml](https://crates.io/crates/toml)           |
| Logging           | [tracing](https://crates.io/crates/tracing)     |
| Auto-registration | [inventory](https://crates.io/crates/inventory) |

## License

Apache 2.0
