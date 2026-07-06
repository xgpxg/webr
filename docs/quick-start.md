# 快速入门

创建第一个 WebR 应用并运行起来。

## 1. 创建项目

```bash
cargo new my-app
cd my-app
```

## 2. 添加依赖

编辑 `Cargo.toml`：

```toml
[dependencies]
webr = { version = "0.1" }
```

## 3. 创建配置文件

`config/application.toml`：

```toml
[server]
port = 8080
```

## 4. 编写代码

`src/main.rs`：

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

// 入口函数
#[webr::main]
async fn main(_app: &mut AppBuilder) -> Result<()> {
    Ok(())
}
```

## 5. 启动

```bash
cargo run
```

如果你看到以下输出，则说明启动成功：

```
2026-07-06T06:29:41.284926Z  INFO webr_web::app: Starting WebR application...
2026-07-06T06:29:41.285005Z  INFO webr_web::app: Configuration loaded: profile=dev, files=[config/application.toml]
2026-07-06T06:29:41.285337Z  INFO webr_web::app: Route mappings:
2026-07-06T06:29:41.285390Z  INFO webr_web::app:   GET / → HelloController
2026-07-06T06:29:41.285633Z  INFO webr_web::app: WebR started on http://0.0.0.0:8080
```

访问 `http://localhost:8080/` 查看结果。

## 项目结构约定

```
my-app/
├── config/
│   └── application.toml    # 配置文件
├── src/
│   └── main.rs              # 应用代码
└── Cargo.toml
```

配置目录查找顺序：`WEBR_CONFIG_DIR` 环境变量 → 可执行文件所在目录向上查找 `config/` → 当前工作目录的 `config/`。
