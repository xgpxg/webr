# 控制器与路由

通过宏定义 HTTP 路由和控制器，零样板代码。

## 基础用法

```rust
#[controller]
pub struct ItemController;

#[controller]
impl ItemController {
    #[get("/items")]
    async fn list(&self) -> Json<Vec<Item>> { todo!() }

    #[get("/items/{id}")]
    async fn get(&self, Path(id): Path<i64>) -> Json<Item> { todo!() }

    #[post("/items")]
    async fn create(&self, Json(dto): Json<CreateDto>) -> StatusCode { todo!() }

    #[put("/items/{id}")]
    async fn update(&self, Path(id): Path<i64>, Json(dto): Json<UpdateDto>) -> Json<Item> { todo!() }

    #[delete("/items/{id}")]
    async fn delete(&self, Path(id): Path<i64>) -> StatusCode { todo!() }

    #[patch("/items/{id}")]
    async fn patch(&self, Path(id): Path<i64>, Json(dto): Json<PatchDto>) -> Json<Item> { todo!() }
}
```

## 路由前缀

在 `impl` 上的 `#[controller(prefix = "...")]` 设置统一前缀：

```rust
#[controller(prefix = "/api")]
impl TodoController {
    #[get("/todos")]       // 实际路由: GET /api/todos
    async fn list(&self) -> Json<Vec<Todo>> { todo!() }
}
```

## 路径参数

使用 `{param}` 语法定义路径参数，`Path<T>` 提取：

```rust
#[get("/users/{user_id}/posts/{post_id}")]
async fn get_post(&self, Path((user_id, post_id)): Path<(i64, i64)>) -> Json<Post> {
    todo!()
}
```

## 支持的 HTTP 方法

| 注解 | HTTP 方法 |
|------|-----------|
| `#[get]` | GET |
| `#[post]` | POST |
| `#[put]` | PUT |
| `#[delete]` | DELETE |
| `#[patch]` | PATCH |

## 控制器结构体字段

控制器 struct 的字段通过 `Inject<T>` 声明依赖，启动时自动注入：

```rust
#[controller]
pub struct UserController {
    user_service: Inject<UserService>,    // 注入业务组件
    config: Inject<AppConfig>,            // 注入配置
}
```

## 路由表日志

启动时框架自动打印路由表：

```
Route mappings:
GET    /items      → ItemController
GET    /items/{id} → ItemController
POST   /items      → ItemController
```
