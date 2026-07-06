# 响应与错误处理

## 响应类型

Handler 可返回多种类型，框架自动转换为 HTTP 响应。

### 直接返回数据

```rust
#[get("/")]
async fn index(&self) -> String {
    "Hello".to_string()
}

#[get("/health")]
async fn health(&self) -> StatusCode {
    StatusCode::OK
}
```

### Json

返回 JSON 响应：

```rust
#[get("/users")]
async fn list(&self) -> Json<Vec<User>> {
    Json(vec![/* ... */])
}
```

### Result / WebrResult

支持 `?` 传播错误：

```rust
use webr::WebrResult;

#[get("/users/{id}")]
async fn get_user(&self, Path(id): Path<i64>) -> WebrResult<Json<User>> {
    let user = self.service.find(id).await
        .ok_or_else(|| Error::Http {
            status: StatusCode::NOT_FOUND,
            message: format!("User {id} not found"),
        })?;
    Ok(Json(user))
}
```

可使用 `WebrResult<T>` 作为 `Result<T, Error>` 的别名。

## 错误处理

### 直接返回 Error

使用 `Error::Http` 构造带状态码和消息的错误：

```rust
use webr::Error;

#[get("/items/{id}")]
async fn get_item(&self, Path(id): Path<i64>) -> Result<Json<Item>, Error> {
    if id > 0 {
        Ok(Json(Item { id, name: "Item".into() }))
    } else {
        Err(Error::Http {
            status: StatusCode::NOT_FOUND,
            message: format!("Item {id} not found"),
        })
    }
}
```

### Error 类型

```rust
pub enum Error {
    Http { status: StatusCode, message: String },  // HTTP 业务错误
    Database(Box<dyn Error + Send + Sync>),          // 数据库错误
    Cache(Box<dyn Error + Send + Sync>),             // 缓存错误
    Internal(String),                                // 内部错误
}
```

错误自动转换为 JSON 响应：`{"code": 404, "message": "Item 42 not found"}`。

### #[derive(HttpError)]

声明式错误定义，自动映射为 HTTP 响应：

```rust
#[derive(Debug, webr::HttpError)]
pub enum UserError {
    #[error(status = 404, message = "User not found")]
    NotFound(i64),

    #[error(status = 409, message = "Email already exists")]
    DuplicateEmail(String),
}

// 在 handler 中使用
async fn get_user(&self, Path(id): Path<i64>) -> Result<Json<User>, UserError> {
    let user = self.service.find(id).await
        .ok_or(UserError::NotFound(id))?;
    Ok(Json(user))
}
```

`HttpError` derive 生成的特性：
- 自动实现 `IntoResponse`，可直接作为返回值
- 自动实现 `From<Self> for Error`，支持 `?` 向上转换为 `Error`

## 统一响应包装

启用 `unified_response` 后，2xx JSON 响应自动包装为标准格式：

```rust
app.unified_response();
```

原始响应：

```json
{"id": 1, "name": "Alice"}
```

包装后：

```json
{"code": 200, "message": "success", "data": {"id": 1, "name": "Alice"}}
```

规则：
- 2xx + JSON 响应 → 包装
- 非 2xx 响应 → 原样透传
- 非 JSON 响应（String、StatusCode 等）→ 原样透传
