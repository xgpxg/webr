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
webr = { path = "/path/to/webr" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

## 3. 创建配置文件

`config/application.toml`：

```toml
[server]
port = 8080

[app]
name = "my-app"
greeting = "Hello from WebR!"
```

## 4. 编写代码

`src/main.rs`：

```rust
use webr::prelude::*;
use webr::{Error, Inject};

// 配置绑定：自动从 [app] 节加载
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub greeting: String,
}

// 业务组件：自动注册到 DI 容器
#[component]
pub struct UserService;

impl UserService {
    pub async fn find_all(&self) -> Vec<String> {
        vec!["Alice".into(), "Bob".into()]
    }
}

// 控制器：通过 Inject<T> 声明依赖
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
    async fn users(&self) -> Json<Vec<String>> {
        Json(self.user_service.find_all().await)
    }
}

// 入口：#[webr::main] 宏包装 tokio runtime + 启动逻辑
#[webr::main]
async fn main(app: &mut AppBuilder) -> Result<(), Error> {
    app.unified_response(); // 可选：启用统一响应包装
    Ok(())
}
```

## 5. 启动

```bash
cargo run
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
