# WebR 框架

一个轻量的 Rust Web 框架，基于 Axum 构建。提供宏驱动控制器、自动依赖注入、多文件配置管理和中间件系统，旨在简化 Rust 中的 Web
开发。

WebR 引入了 Java 系的框架中的 `DI`、`Controller`、`Component`和自动配置等概念，在保证性能的同时，帮助开发者更快速的使用 Rust
构建 Web 应用。

## 特性概览

- **宏驱动控制器** — `#[controller]` + `#[get]` / `#[post]` 等，零样板代码
- **依赖注入** — `#[component]` 声明组件，`Inject<T>` 自动注入
- **配置管理** — 多 TOML 文件 + Profile 切换 + 环境变量覆盖，`#[config]` 绑定注入
- **中间件系统** — 全局/路径范围中间件，内置 CORS、日志、Panic 恢复、统一响应
- **认证机制** — `AuthMiddleware` + `Authenticator` trait，路径级权限控制
- **请求验证** — `Json` / `Query` / `Form` 提取器自动验证
- **文件上传与 SSE** — Multipart 提取器、FileResponse、SseResponse
- **声明式错误处理** — `#[derive(HttpError)]` 将业务错误映射到 HTTP 状态码
- **数据库** — 连接池、`#[sql]` 动态查询（支持 MyBatis 风格标签）、`#[tx]` 事务管理
- **缓存** — Memory / Sled / Redis 三种后端，统一 API
