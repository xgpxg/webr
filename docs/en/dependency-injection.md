# Dependency Injection

WebR's IoC container manages the lifecycle of all components, automatically resolving and injecting dependencies at startup.

## Core concepts

- **`#[component]`** — Marks a struct as a managed component, automatically registered in the IoC container
- **`#[controller]`** — Marks a controller, which is also a component, used for declarative routing
- **`#[config]`** — Marks a configuration type, also a component, values loaded from TOML files
- **`Inject<T>`** — Injection smart pointer, automatically resolves and holds component instances

## Declaring components

### Basic component

```rust
#[component]
pub struct UserService;

impl UserService {
    pub async fn find_all(&self) -> Vec<String> {
        vec!["Alice".into(), "Bob".into()]
    }
}
```

### Component with dependencies

Declare dependencies on other components using `Inject<T>` in component fields:

```rust
#[component]
pub struct OrderService {
    user_service: Inject<UserService>,
    inventory_service: Inject<InventoryService>,
}
```

### Injection in controllers

```rust
#[controller]
pub struct OrderController {
    order_service: Inject<OrderService>,
    config: Inject<AppConfig>,
}
```

## Injecting configuration

Structs marked with `#[config(prefix = "...")]` are also injectable components:

```rust
#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    pub greeting: String,
}

#[controller]
pub struct MyController {
    config: Inject<AppConfig>,
}
```

## Manual registration

Without macros, register manually via `app.provide()`:

```rust
use webr::db::DbPool;

let pool = DbPool::from_config(&config).await?;
app.provide(pool)?;  // Register to DI container, then injectable via Inject<DbPool>
```

## Dependency resolution principles

1. **Registration phase**: `#[component]` / `#[controller]` macros register component descriptors at compile time via `inventory`
2. **Build phase**: `AppBuilder::build()` traverses all registered descriptors, executes Kahn's topological sort to determine instantiation order
3. **Instantiation**: Components are created in sorted order, each component resolves its `Inject<T>` fields from the container during construction
4. **Injection**: Once created, components are stored in the container for use by subsequent components and controllers

**Circular dependency detection**: Topological sorting automatically detects circular dependencies. If a cycle exists, an error is reported at startup:

```
Circular dependency detected among: UserService, OrderService
```

## `Inject<T>` API

`Inject<T>` implements `Deref<Target=T>`, so you can directly call T's methods:

```rust
#[controller]
pub struct UserController {
    user_service: Inject<UserService>,
}

impl UserController {
    #[get("/users")]
    async fn list(&self) -> Json<Vec<User>> {
        let users = self.user_service.find_all().await; // Transparent call
        Json(users)
    }
}
```

`Inject<T>` also provides:

- `.arc()` — Get `Arc<T>` reference
- `.clone()` — Shallow clone (shares the same instance, reference count incremented)
