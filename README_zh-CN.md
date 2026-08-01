<p align="center">
  <img src="docs/public/logo.png" alt="WebR Logo" width="150">
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="https://xgpxg.github.io/webr">文档</a>
</p>
    

WebR 将 Spring Boot 的开发体验带入 Rust 生态 ——
宏驱动的控制器、自动依赖注入、配置管理、中间件系统、ORM、缓存，构建于 [Axum](https://github.com/tokio-rs/axum) 之上。

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

如果一切正常，访问 http://localhost:8080 即可看到 `hello world`。

## 文档

完整文档见 [WebR Document](https://xgpxg.github.io/webr)。

- [快速开始](https://xgpxg.github.io/webr/quick-start)
- [依赖注入](https://xgpxg.github.io/webr/dependency-injection)
- [配置](https://xgpxg.github.io/webr/configuration)
- [控制器与路由](https://xgpxg.github.io/webr/controllers-routing)
- [请求处理与校验](https://xgpxg.github.io/webr/request-handling)
- [响应与错误处理](https://xgpxg.github.io/webr/response-error)
- [中间件与认证鉴权](https://xgpxg.github.io/webr/middleware)
- [文件上传与下载](https://xgpxg.github.io/webr/file-upload)
- [SSE](https://xgpxg.github.io/webr/sse)
- [数据库](https://xgpxg.github.io/webr/database)
- [缓存](https://xgpxg.github.io/webr/cache)
- [性能报告](https://xgpxg.github.io/webr/performance)

## 示例

在 [examples](examples) 目录下，提供了一些示例项目，可以通过这些示例来快速了解 WebR 的使用。

| 示例                                    | 说明                               |
|---------------------------------------|----------------------------------|
| [`hello-world`](examples/hello-world) | 控制器、依赖注入、配置绑定、统一响应               |
| [`middleware`](examples/middleware)   | 自定义认证中间件、路径范围路由、CORS             |
| [`file-upload`](examples/file-upload) | 多文件上传、文件下载、内联预览                  |
| [`sse`](examples/sse)                 | Server-Sent Events 服务端推送         |
| [`orm`](examples/orm)                 | 实体 CRUD、`#[sql]` 动态查询、`#[tx]` 事务 |
| [`cache`](examples/cache)             | 缓存模块使用                           |
| [`axum-bench`](examples/axum-bench)   | 原生 axum 与 WebR 性能对比基准            |

运行示例：

```bash
cd examples/hello-world
cargo run
```

## 项目结构

```
webr/
├── crates/
│   ├── webr-core/        # 核心基础：DI、配置、上下文、Inject、Error
│   ├── webr-web/         # Web 层：AppBuilder、提取器、中间件、响应、路由
│   ├── webr-db/          # 数据库：连接池、事务、ORM 支持
│   ├── webr-cache/       # 缓存：内存 / Sled / Redis 多后端
│   ├── webr-macros/      # 过程宏：controller、component、config、entity、sql、tx、HttpError
│   └── webr-middleware/  # 中间件：认证、鉴权、请求体缓存
├── examples/             # 示例应用
└── src/lib.rs            # Umbrella crate，统一导出所有公共 API
```

## 许可证

Apache 2.0
