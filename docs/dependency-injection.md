# 依赖注入

WebR 的 IoC 容器管理所有组件的生命周期，启动时自动完成依赖解析和注入。

## 核心概念

- **`#[component]`** — 标记一个结构体为可管理组件，自动注册到 IoC 容器
- **`#[controller]`** — 标记控制器，也是组件的一种，附带路由功能
- **`#[config]`** — 标记配置类型，也是组件，值从 TOML 文件加载
- **`Inject<T>`** — 注入智能指针，自动解析并持有组件实例

## 声明组件

### 基础组件

```rust
#[component]
pub struct UserService;

impl UserService {
    pub async fn find_all(&self) -> Vec<String> {
        vec!["Alice".into(), "Bob".into()]
    }
}
```

### 带依赖的组件

组件字段中用 `Inject<T>` 声明对其他组件的依赖：

```rust
#[component]
pub struct OrderService {
    user_service: Inject<UserService>,
    inventory_service: Inject<InventoryService>,
}
```

### 控制器中的注入

```rust
#[controller]
pub struct OrderController {
    order_service: Inject<OrderService>,
    config: Inject<AppConfig>,
}
```

## 注入配置

通过 `#[config(prefix = "...")]` 标记的结构体也是可注入的组件：

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

## 手动注册

不使用宏时，可通过 `app.provide()` 手动注册：

```rust
use webr::db::DbPool;

let pool = DbPool::from_config(&config).await?;
app.provide(pool)?;  // 注册到 DI 容器，之后可通过 Inject<DbPool> 注入
```

## 依赖解析原理

1. **注册阶段**：`#[component]` / `#[controller]` 宏在编译期通过 `inventory` 注册组件描述符
2. **构建阶段**：`AppBuilder::build()` 遍历所有注册的描述符，执行 Kahn 拓扑排序确定实例化顺序
3. **实例化**：按排序后的顺序依次创建组件实例，每个组件构造时从容器中解析其 `Inject<T>` 字段
4. **注入**：组件创建完成后存入容器，供后续组件和控制器使用

**循环依赖检测**：拓扑排序时自动检测循环依赖，如有循环则启动时报错：

```
Circular dependency detected among: UserService, OrderService
```

## `Inject<T>` API

`Inject<T>` 实现了 `Deref<Target=T>`，可直接调用 T 的方法：

```rust
#[controller]
pub struct UserController {
    user_service: Inject<UserService>,
}

impl UserController {
    #[get("/users")]
    async fn list(&self) -> Json<Vec<User>> {
        let users = self.user_service.find_all().await; // 透明调用
        Json(users)
    }
}
```

`Inject<T>` 还提供：

- `.arc()` — 获取 `Arc<T>` 引用
- `.clone()` — 浅克隆（共享同一实例，引用计数增加）
