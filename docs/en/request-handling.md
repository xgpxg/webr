# Request Handling & Validation

WebR provides multiple request data extractors with automatic serialization and validation.

## Extractors overview

| Extractor | Data source | Use case |
|-----------|-------------|----------|
| `Json<T>` | Request body (JSON) | REST API request body |
| `Query<T>` | URL query parameters | GET request parameters |
| `Path<T>` | URL path parameters | `/users/{id}` |
| `Form<T>` | Request body (form) | `application/x-www-form-urlencoded` |
| `Header<T>` | Request headers | Custom header extraction |
| `HeaderMap` | Raw request headers | Arbitrary header reading |
| `Multipart` | Request body (multipart) | File uploads |

## Json

Parses `Content-Type: application/json` request body:

```rust
#[derive(Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub age: u8,
}

#[post("/users")]
async fn create(&self, Json(body): Json<CreateUser>) -> Json<User> {
    // body is already deserialized
    Json(user_service.create(body).await)
}
```

## Query

Parses URL query parameters `?page=1&size=10`:

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

Extracts URL path parameters `{id}`:

```rust
#[get("/users/{id}")]
async fn get_user(&self, Path(id): Path<i64>) -> Json<User> {
    // id = i64 value extracted from path
    todo!()
}

// Multiple path parameters
#[get("/users/{user_id}/posts/{post_id}")]
async fn get_post(&self, Path((user_id, post_id)): Path<(i64, i64)>) -> Json<Post> {
    todo!()
}
```

## Form

Parses `application/x-www-form-urlencoded` forms:

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

Extracts request headers, header keys auto-converted to snake_case to match field names:

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

Directly access raw request headers:

```rust
use webr::HeaderMapExt;

async fn handler(headers: HeaderMap) -> String {
    let token = headers.get_str("authorization").unwrap_or("");
    let page: i32 = headers.get_parsed("x-page").unwrap_or(1);
    format!("token: {token}, page: {page}")
}
```

## Request body validation

Derive `Validate` on DTOs, extractors automatically run validation:

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
    // dto has passed validation, otherwise returns 422 Unprocessable Entity
    todo!()
}
```

Validation failures automatically return `422 Unprocessable Entity`, with error details for each field in the response body.
