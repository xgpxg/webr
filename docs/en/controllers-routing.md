# Controllers & Routing

Define HTTP routes and controllers via macros, zero boilerplate.

## Basic usage

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

## Route prefix

Use `#[controller(prefix = "...")]` on the `impl` block to set a unified prefix:

```rust
#[controller(prefix = "/api")]
impl TodoController {
    #[get("/todos")]       // Actual route: GET /api/todos
    async fn list(&self) -> Json<Vec<Todo>> { todo!() }
}
```

## Path parameters

Use `{param}` syntax to define path parameters, extracted with `Path<T>`:

```rust
#[get("/users/{user_id}/posts/{post_id}")]
async fn get_post(&self, Path((user_id, post_id)): Path<(i64, i64)>) -> Json<Post> {
    todo!()
}
```

## Controller struct fields

Controller struct fields declare dependencies via `Inject<T>`, automatically injected at startup:

```rust
#[controller]
pub struct UserController {
    user_service: Inject<UserService>,    // Inject business component
    config: Inject<AppConfig>,            // Inject configuration
}
```

## Route table logging

The framework automatically prints the route table at startup:

```
Route mappings:
GET    /items      → ItemController
GET    /items/{id} → ItemController
POST   /items      → ItemController
```
