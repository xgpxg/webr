# Middleware

WebR's middleware system is based on Axum's middleware, with a simple wrapper supporting global, path-scoped, and exclusion-based registration.

## Defining middleware

Implement the `Middleware` trait:

```rust
use webr::async_trait::async_trait;
use webr::middleware::{Middleware, Next};
use axum::extract::Request;
use axum::response::Response;

pub struct MyMiddleware;

#[async_trait]
impl Middleware for MyMiddleware {
    async fn handle(&self, request: Request, next: Next) -> Response {
        // Pre-processing
        println!("before handler");

        let response = next.run(request).await;

        // Post-processing
        println!("after handler");
        response
    }
}
```

## Registering middleware

### Global middleware

Applies to all routes:

```rust
#[webr::main]
async fn main(app: &mut AppBuilder) -> Result<(), Error> {
    app.middleware(LoggerMiddleware);
    app.middleware(PanicRecovery);
    app.middleware(CorsMiddleware::new().allow_origin("*"));
    // Enable unified response wrapping (convenience method)
    app.unified_response();
    Ok(())
}
```

### Path-scoped middleware

Only applies to routes matching the specified path pattern:

```rust
// Prefix match: all routes under /api/**
app.middleware_for("/api/**", RequireAuth);

// Exact match: only /admin path
app.middleware_for("/admin", AdminOnlyMiddleware);
```

### Exclusion mode middleware

Applies to all routes except the matched path:

```rust
// LoggerMiddleware for all requests except /health
app.middleware_except("/health", LoggerMiddleware);
```

### Execution order

Middleware executes in registration order, forming an onion model:

```
Middleware1 → Middleware2 → ... → Handler → ... → Middleware2 → Middleware1
```

## Built-in middleware

### LoggerMiddleware

Request logging: method, path, status code, duration.

```rust
app.middleware(LoggerMiddleware);
```

Log output:

```
-> GET /api/users
<- GET /api/users 200 OK (1.23ms)
```

### CorsMiddleware

CORS configuration with Builder pattern chain calls.

```rust
// Default: allow all origins
app.middleware(CorsMiddleware::new());

// Custom configuration
app.middleware(
CorsMiddleware::new()
.allow_origin("https://example.com")
.allow_methods(["GET", "POST"])
.allow_headers(["Content-Type", "Authorization"])
.allow_credentials(true)
.max_age(3600),
);
```

Defaults:

| Option | Default |
|--------|---------|
| allow_origin | `*` |
| allow_methods | `GET,POST,PUT,DELETE,PATCH,OPTIONS` |
| allow_headers | `Content-Type,Authorization` |
| allow_credentials | `false` |

Note: `allow_credentials(true)` and `allow_origin("*")` are mutually exclusive; setting both will panic.

### PanicRecovery

Catches panics in handlers, returns 500 error to prevent process crash.

```rust
app.middleware(PanicRecovery);
```

### UnifiedResponse

Wraps 2xx JSON responses into a unified format `{"code": 200, "message": "success", "data": ...}`.

```rust
app.unified_response();  // Recommended
// Equivalent to: app.middleware(UnifiedResponse);
```

Rules:

| Response type | Handling |
|--------------|----------|
| 2xx + JSON | Wrapped to standard format |
| Non-2xx | Passed through as-is |
| Non-JSON | Passed through as-is |

### AuthMiddleware

Authentication middleware, used with the `Authenticator` trait:

```rust
// Global authentication
app.middleware(AuthMiddleware::new(JwtAuth));

// Exclude public paths
app.middleware_except("/login", AuthMiddleware::new(JwtAuth));
```

See [Authentication & Authorization](#authentication--authorization) section.

### CachedBodyMiddleware

Caches request body in memory, solving the problem of body being consumed only once. Must be registered before middleware that reads the body:

```rust
app.middleware(CachedBodyMiddleware);
app.middleware(AuthMiddleware::new(WebhookAuth)); // Needs to read body
```

## Authentication & Authorization

### Authenticator

Implement the `Authenticator` trait to define authentication logic:

```rust
use webr::Authenticator;

struct JwtAuth;

#[async_trait]
impl Authenticator for JwtAuth {
    type Identity = UserInfo;

    async fn authenticate(
        &self,
        headers: &HeaderMap,
        body: Option<&Bytes>,
    ) -> Result<UserInfo, AuthError> {
        let token = headers.get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::new("Missing token"))?;
        // Validate token, return user identity
        decode_jwt(token).map_err(|e| AuthError::new(e.to_string()))
    }
}
```

### CurrentUser

Extract the authenticated user identity in controllers:

```rust
#[controller]
impl ApiController {
    #[get("/profile")]
    async fn profile(&self, CurrentUser(user): CurrentUser<UserInfo>) -> Json<UserInfo> {
        Json(user)
    }
}
```

### Guard (authorization guard)

Implement the `Guard` trait for fine-grained permission checks:

```rust
struct AdminGuard;

#[async_trait]
impl Guard for AdminGuard {
    async fn check(&self, req: &Request) -> Result<(), Error> {
        let user = req.extensions().get::<UserInfo>()
            .ok_or_else(|| Error::Http {
                status: StatusCode::UNAUTHORIZED,
                message: "Not authenticated".into(),
            })?;
        if user.role != "admin" {
            return Err(Error::Http {
                status: StatusCode::FORBIDDEN,
                message: "Admin access required".into(),
            });
        }
        Ok(())
    }
}

// Register
app.middleware_for("/admin/**", GuardMiddleware::new(AdminGuard));
```
