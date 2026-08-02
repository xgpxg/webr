<p align="center">
  <img src="docs/public/logo.png" alt="WebR Logo" width="150">
</p>

<p align="center">
  <a href="README_zh-CN.md">中文</a> · <a href="https://xgpxg.github.io/webr/en">Document</a>
</p>


WebR brings the developer experience of Spring Boot to the Rust ecosystem — macro-driven controllers, automatic
dependency injection, ORM and cache, configuration management, and a built-in middleware system, all built on top
of <a href="https://github.com/tokio-rs/axum">Axum</a>.

```rust
use webr::prelude::*;

#[controller]
pub struct HelloController;

#[controller]
impl HelloController {
    #[get("/")]
    async fn index(&self) -> String {
        "hello world".to_string()
    }
}

#[webr::main]
async fn main(_app: &mut AppBuilder) -> Result<()> {
    Ok(())
}
```

If everything goes well, visit http://localhost:8080 and you will see `hello world`.

## Documentation

Full documentation is available at [WebR Document](https://xgpxg.github.io/webr).

- [Quick Start](https://xgpxg.github.io/webr/quick-start)
- [Dependency Injection](https://xgpxg.github.io/webr/dependency-injection)
- [Configuration](https://xgpxg.github.io/webr/configuration)
- [Routing & Controllers](https://xgpxg.github.io/webr/controllers-routing)
- [Request Handling & Validation](https://xgpxg.github.io/webr/request-handling)
- [Response & Error Handling](https://xgpxg.github.io/webr/response-error)
- [Middleware & Authentication](https://xgpxg.github.io/webr/middleware)
- [File Upload & Download](https://xgpxg.github.io/webr/file-upload)
- [SSE](https://xgpxg.github.io/webr/sse)
- [Database](https://xgpxg.github.io/webr/database)
- [Cache](https://xgpxg.github.io/webr/cache)
- [Performance](https://xgpxg.github.io/webr/performance)

## Examples

The [examples](examples) directory contains some example projects that you can use to quickly get familiar with WebR.

| Example                               | Description                                                 |
|---------------------------------------|-------------------------------------------------------------|
| [`hello-world`](examples/hello-world) | Controllers, DI, config binding, unified response           |
| [`middleware`](examples/middleware)   | Custom auth middleware, path-scoped routing, CORS           |
| [`file-upload`](examples/file-upload) | Multi-file upload, file download, inline preview            |
| [`sse`](examples/sse)                 | Server-Sent Events streaming                                |
| [`orm`](examples/orm)                 | Entity CRUD, `#[sql]` dynamic queries, `#[tx]` transactions |
| [`cache`](examples/cache)             | Cache module usage                                          |
| [`axum-bench`](examples/axum-bench)   | Raw axum vs WebR performance benchmark                      |

Run an example:

```bash
cd examples/hello-world
cargo run
```

## Project Structure

```
webr/
├── crates/
│   ├── webr-core/        # Core primitives: DI, config, context, Inject, Error
│   ├── webr-web/         # Web layer: AppBuilder, extractors, middleware, response, router
│   ├── webr-db/          # Database: connection pool, transactions, ORM support
│   ├── webr-cache/       # Cache: Memory / Sled / Redis backends
│   ├── webr-macros/      # Procedural macros: controller, component, config, entity, sql, tx, HttpError
│   └── webr-middleware/  # Middleware: authentication, authorization, request body caching
├── examples/             # Example applications
└── src/lib.rs            # Umbrella crate, unified re-export of all public APIs
```

## License

Apache 2.0
