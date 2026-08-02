# Configuration System

WebR supports multi-file TOML configuration, profile switching, and environment variable overrides.

## Loading priority

Configuration sources are loaded in priority order (later overrides earlier):

| Priority | Source | Description |
|:--------:|--------|-------------|
| 1 | Built-in defaults | `default()` in structs like ServerConfig |
| 2 | `config/application.toml` | Base configuration |
| 3 | `config/application-{profile}.toml` | Profile config, default `dev` |
| 4 | `WEBR_` prefixed environment variables | e.g. `WEBR_SERVER_PORT=9090` |

## Profile switching

Set the `WEBR_PROFILE` environment variable to switch environments:

```bash
WEBR_PROFILE=prod cargo run
```

This loads `config/application-prod.toml`, whose values override matching keys in `application.toml`.

## `#[config]` binding

Use `#[config(prefix = "section")]` to bind a struct to a TOML section, making it injectable:

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

Use `Inject<AppConfig>` in controllers or components to obtain the configuration:

```rust
#[controller]
pub struct MyController {
    config: Inject<AppConfig>,
}
```

## Environment variable overrides

Environment variables with the `WEBR_` prefix can override any configuration key. Naming convention: double underscore `__` represents hierarchy separator.

```bash
# Equivalent to setting [server] port = 9090
export WEBR_SERVER_PORT=9090

# Equivalent to setting [datasource] url = "postgres://..."
export WEBR_DATASOURCE__URL="postgres://user:pass@localhost/db"
```

Values are auto-inferred: `i64` → integer, `f64` → float, `true`/`false` → boolean, rest → string.

## Built-in configuration sections

### [server]

```toml
[server]
port = 8080               # Listen port, default 8080
host = "0.0.0.0"          # Listen address, default 0.0.0.0
max_body_size = 2097152   # Request body limit (bytes), default 2MB
```

### [log]

```toml
[log]
level = "info"            # Log level, default "info"
```

## Manual configuration access

Read any configuration in the `#[webr::main]` function via `app.config()`:

```rust
#[webr::main]
async fn main(app: &mut AppBuilder) -> Result<(), Error> {
    // Manually parse a configuration section
    let app_name: String = app.config()
        .get("app")
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(())
}
```
