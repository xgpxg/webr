# WebR Framework

A Spring Boot-inspired web framework for Rust, built on Axum. Provides macro-driven controllers, automatic dependency injection, multi-file configuration management, and middleware system.

## Quick Start

| Module | Description |
|--------|-------------|
| [Quick Start](quick-start.md) | Create your first WebR application |
| [Configuration](configuration.md) | Multi-file config, Profile, environment variables |
| [Controllers & Routing](controllers-routing.md) | Controller definition, route annotations |
| [Dependency Injection](dependency-injection.md) | Component, Inject, lifecycle |
| [Middleware](middleware.md) | Global/local middleware, CORS, logging, auth |
| [Request Handling](request-handling.md) | Body extraction, query params, forms, headers |
| [Response & Error](response-error.md) | Unified response, error mapping, HttpError derive |
| [File Upload & SSE](file-upload-sse.md) | File upload/download, SSE server push |
| [Database](database.md) | Connection pool, entities, dynamic SQL, transactions |
| [Cache](cache.md) | Memory/Sled/Redis backends, KV operations |

## Features

- **Macro-driven Controllers** — `#[controller]` + `#[get]` / `#[post]`, zero boilerplate
- **Dependency Injection** — `#[component]` + `Inject<T>` auto-injection with topological sorting
- **Configuration Management** — Multi-TOML files + Profile switching + env var override
- **Middleware System** — Global/path-scoped middleware with CORS, logging, panic recovery
- **Authentication** — `AuthMiddleware` + `Authenticator` trait for path-level access control
- **Request Validation** — `Json` / `Query` / `Form` extractors with auto-validation
- **File Upload & SSE** — Multipart extractor, FileResponse, SseResponse
- **Declarative Error Handling** — `#[derive(HttpError)]` maps business errors to HTTP status codes
- **Database** — Connection pool, `#[sql]` dynamic queries (MyBatis-style tags), `#[tx]` transactions
- **Cache** — Memory / Sled / Redis backends with unified API

---

[中文文档 →](README_zh-CN.md)
