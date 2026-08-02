# Quick Start

Create your first WebR application and get it running.

## 1. Create a project

```bash
cargo new my-app
cd my-app
```

## 2. Add dependencies

Edit `Cargo.toml`:

```toml
[dependencies]
webr = { version = "0.1", features = ["web"] }
```

## 3. Create configuration file

`config/application.toml`:

```toml
[server]
port = 8080
```

## 4. Write code

`src/main.rs`:

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

// Entry point
#[webr::main]
async fn main(_app: &mut AppBuilder) -> Result<()> {
    Ok(())
}
```

## 5. Run

```bash
cargo run
```

If you see the following output, the application started successfully:

```
2026-07-06T06:29:41.284926Z  INFO webr_web::app: Starting WebR application...
2026-07-06T06:29:41.285005Z  INFO webr_web::app: Configuration loaded: profile=dev, files=[config/application.toml]
2026-07-06T06:29:41.285337Z  INFO webr_web::app: Route mappings:
2026-07-06T06:29:41.285390Z  INFO webr_web::app:   GET / → HelloController
2026-07-06T06:29:41.285633Z  INFO webr_web::app: WebR started on http://0.0.0.0:8080
```

Visit `http://localhost:8080/` to see the result.

## Project structure conventions

```
my-app/
├── config/
│   └── application.toml    # Configuration files
├── src/
│   └── main.rs              # Application code
└── Cargo.toml
```

Configuration directory lookup order: `WEBR_CONFIG_DIR` environment variable → `config/` directory found by walking up from the executable's directory → `config/` in the current working directory.
