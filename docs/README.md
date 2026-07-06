# WebR 框架使用文档

WebR 是一个受 Spring Boot 启发的 Rust Web 框架，基于 Axum 构建。通过宏驱动控制器、自动依赖注入、多文件配置管理和中间件系统，提供类似 Spring Boot 的开发体验。

## 文档目录

| 模块 | 说明 |
|------|------|
| [快速入门](quick-start.md) | 创建第一个 WebR 应用 |
| [配置系统](configuration.md) | 多文件配置、Profile、环境变量覆盖 |
| [控制器与路由](controllers-routing.md) | Controller 定义、路由注解、路径参数 |
| [依赖注入](dependency-injection.md) | Component、Inject、生命周期 |
| [中间件](middleware.md) | 全局/局部中间件、CORS、日志、认证 |
| [请求处理与验证](request-handling.md) | 请求体提取、查询参数、表单、请求头、验证 |
| [响应与错误处理](response-error.md) | 统一响应、错误映射、HttpError derive |
| [文件下载与 SSE](file-upload-sse.md) | 文件上传下载、SSE 服务端推送 |
| [数据库模块](database.md) | 连接池、实体、动态 SQL、事务、分页 |
| [缓存模块](cache.md) | Memory/Sled/Redis 后端、KV 操作、数据结构 |

## 特性概览

- **宏驱动控制器** — `#[controller]` + `#[get]` / `#[post]` 等，零样板代码
- **依赖注入** — `#[component]` 声明组件，`Inject<T>` 自动注入，启动时拓扑排序
- **配置管理** — 多 TOML 文件 + Profile 切换 + 环境变量覆盖，`#[config]` 绑定注入
- **中间件系统** — 全局/路径范围中间件，内置 CORS、日志、Panic 恢复、统一响应
- **认证机制** — `AuthMiddleware` + `Authenticator` trait，路径级权限控制
- **请求验证** — `Json` / `Query` / `Form` 提取器自动验证
- **文件上传与 SSE** — Multipart 提取器、FileResponse、SseResponse
- **声明式错误处理** — `#[derive(HttpError)]` 将业务错误映射到 HTTP 状态码
- **数据库** — 连接池、`#[sql]` 动态查询（支持 MyBatis 风格标签）、`#[tx]` 事务管理
- **缓存** — Memory / Sled / Redis 三种后端，统一 API
