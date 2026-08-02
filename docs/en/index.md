# WebR Framework

A lightweight Rust web framework built on top of Axum. Provides macro-driven controllers, automatic dependency injection, multi-file configuration management, and a middleware system, aiming to simplify web development in Rust.

WebR introduces concepts from the Java ecosystem such as `DI`, `Controller`, `Component`, and auto-configuration, helping developers build web applications in Rust more rapidly while maintaining performance.

## Feature Overview

- **Macro-driven routing** — `#[controller]` / `#[get]` / `#[post]`, zero boilerplate
- **DI + configuration management** — `#[component]` / `#[config]` declarative injection, multi-profile switching
- **Middleware** — Global/path-scoped middleware, built-in CORS, logging, panic recovery
- **Request handling** — JSON / Query / Form / Header extractors, multipart uploads, SSE push
- **Error handling** — `#[derive(HttpError)]` for quick HTTP status code mapping
- **Database** — Connection pool, `#[sql]` dynamic queries, `#[tx]` transactions
- **Cache** — Unified API for Memory / Sled / Redis
