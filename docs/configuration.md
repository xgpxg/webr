# 配置系统

WebR 支持多文件 TOML 配置、Profile 切换和环境变量覆盖。

## 加载优先级

配置来源按优先级排列（后者覆盖前者）：

| 优先级 | 来源 | 说明 |
|:------:|------|------|
| 1 | 内置默认值 | ServerConfig 等结构体内部的 `default()` |
| 2 | `config/application.toml` | 基准配置 |
| 3 | `config/application-{profile}.toml` | Profile 配置，默认 `dev` |
| 4 | `WEBR_` 前缀环境变量 | 如 `WEBR_SERVER_PORT=9090` |

## Profile 切换

设置 `WEBR_PROFILE` 环境变量切换环境：

```bash
WEBR_PROFILE=prod cargo run
```

此配置会加载 `config/application-prod.toml`，其中的值覆盖 `application.toml` 的同名键。

## `#[config]` 配置绑定

通过 `#[config(prefix = "section")]` 将结构体绑定到 TOML 的指定节，使其可注入：

```toml
[app]
name = "my-app"
version = "1.0.0"
greeting = "Hello!"
```

```rust
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub greeting: String,
}
```

使用 `Inject<AppConfig>` 在控制器或组件中获取配置：

```rust
#[controller]
pub struct MyController {
    config: Inject<AppConfig>,
}
```

## 环境变量覆盖

`WEBR_` 前缀的环境变量可覆盖任意配置项。命名规则：双下划线 `__` 表示层级分隔。

```bash
# 等价于设置 [server] port = 9090
export WEBR_SERVER_PORT=9090

# 等价于设置 [datasource] url = "postgres://..."
export WEBR_DATASOURCE__URL="postgres://user:pass@localhost/db"
```

值自动推断类型：`i64` → 整数，`f64` → 浮点数，`true`/`false` → 布尔值，其余→字符串。

## 内置配置节

### [server]

```toml
[server]
port = 8080               # 监听端口，默认 8080
host = "0.0.0.0"          # 监听地址，默认 0.0.0.0
max_body_size = 2097152   # 请求体上限（字节），默认 2MB
```

### [log]

```toml
[log]
level = "info"            # 日志级别，默认 "info"
```

## 手动获取配置

在 `#[webr::main]` 函数中通过 `app.config()` 直接读取任意配置：

```rust
#[webr::main]
async fn main(app: &mut AppBuilder) -> Result<(), Error> {
    // 手动解析配置节
    let app_name: String = app.config()
        .get("app")
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(())
}
```
