# 请求处理与验证

WebR 提供多种请求数据提取器，支持自动序列化和验证。

## 提取器一览

| 提取器 | 数据来源 | 适用场景 |
|--------|----------|----------|
| `Json<T>` | 请求体 (JSON) | REST API 请求体 |
| `Query<T>` | URL 查询参数 | GET 请求参数 |
| `Path<T>` | URL 路径参数 | `/users/{id}` |
| `Form<T>` | 请求体 (表单) | `application/x-www-form-urlencoded` |
| `Header<T>` | 请求头 | 自定义 Header 提取 |
| `HeaderMap` | 原始请求头 | 任意 Header 读取 |
| `Multipart` | 请求体 (multipart) | 文件上传 |

## Json

解析 `Content-Type: application/json` 请求体：

```rust
#[derive(Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub age: u8,
}

#[post("/users")]
async fn create(&self, Json(body): Json<CreateUser>) -> Json<User> {
    // body 已自动反序列化
    Json(user_service.create(body).await)
}
```

## Query

解析 URL 查询参数 `?page=1&size=10`：

```rust
#[derive(Deserialize)]
pub struct PageQuery {
    pub page: u32,
    pub size: u32,
}

#[get("/items")]
async fn list(&self, Query(q): Query<PageQuery>) -> Json<Vec<Item>> {
    // q.page, q.size
    todo!()
}
```

## Path

提取 URL 路径参数 `{id}`：

```rust
#[get("/users/{id}")]
async fn get_user(&self, Path(id): Path<i64>) -> Json<User> {
    // id = 路径中提取的 i64 值
    todo!()
}

// 多路径参数
#[get("/users/{user_id}/posts/{post_id}")]
async fn get_post(&self, Path((user_id, post_id)): Path<(i64, i64)>) -> Json<Post> {
    todo!()
}
```

## Form

解析 `application/x-www-form-urlencoded` 表单：

```rust
#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[post("/login")]
async fn login(&self, Form(form): Form<LoginForm>) -> Json<Token> {
    todo!()
}
```

## Header

提取请求头，header key 自动转 snake_case 匹配字段名：

```rust
#[derive(Deserialize)]
pub struct AuthHeaders {
    pub authorization: String,
    #[serde(rename = "x-request-id")]
    pub request_id: Option<String>,
}

#[get("/me")]
async fn me(&self, Header(h): Header<AuthHeaders>) -> Json<User> {
    // h.authorization, h.request_id
    todo!()
}
```

## HeaderMap

直接访问原始请求头：

```rust
use webr::HeaderMapExt;

async fn handler(headers: HeaderMap) -> String {
    let token = headers.get_str("authorization").unwrap_or("");
    let page: i32 = headers.get_parsed("x-page").unwrap_or(1);
    format!("token: {token}, page: {page}")
}
```

## 请求体验证

在 DTO 上派生 `Validate`，提取器自动执行验证：

```rust
#[derive(Deserialize, Validate)]
pub struct CreateUserDto {
    #[validate(length(min = 1, max = 50))]
    pub name: String,

    #[validate(email)]
    pub email: String,

    #[validate(range(min = 18, max = 150))]
    pub age: u8,
}

#[post("/users")]
async fn create(&self, Json(dto): Json<CreateUserDto>) -> Json<User> {
    // dto 已验证通过，否则返回 422 Unprocessable Entity
    todo!()
}
```

验证失败自动返回 `422 Unprocessable Entity`，响应体中包含各字段的错误详情。
