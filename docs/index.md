# WebR 框架

一个轻量的 Rust Web 框架，基于 Axum 构建。提供宏驱动控制器、自动依赖注入、多文件配置管理和中间件系统，旨在简化 Rust 中的 Web
开发。

WebR 引入了 Java 系的框架中的 `DI`、`Controller`、`Component`和自动配置等概念，在保证性能的同时，帮助开发者更快速的使用 Rust
构建 Web 应用。

## 特性概览

- **宏驱动路由** — `#[controller]` / `#[get]` / `#[post]`，零样板
- **DI + 配置管理** — `#[component]` / `#[config]` 声明式注入，多 Profile 切换
- **中间件** — 全局/路径级中间件，内置 CORS、日志、Panic 恢复
- **请求处理** — JSON / Query / Form / Header 提取器，Multipart 上传，SSE 推送
- **错误处理** — `#[derive(HttpError)]` 快速映射 HTTP 状态码
- **数据库** — 连接池、`#[sql]` 动态查询、`#[tx]` 事务
- **缓存** — Memory / Sled / Redis 统一 API
