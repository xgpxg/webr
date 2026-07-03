use std::sync::Arc;

use axum::extract::{FromRequestParts, Request};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use webr_core::error::Error;
use webr_core::middleware::{Middleware, Next};

use crate::cached_body::CachedBody;

/// 认证错误，由 [`Authenticator`] 返回，自动响应 `401 Unauthorized`。
#[derive(Debug)]
pub struct AuthError(pub String);

impl AuthError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AuthError {}

/// 认证器 trait，实现具体的认证策略（JWT、API Key、Session 等）。
///
/// 认证成功后产出的 `Identity` 类型会存入 request extensions，
/// 下游通过 [`CurrentUser<T>`] 提取。
///
/// # Example
///
/// ```rust
/// struct JwtAuth;
///
/// #[async_trait::async_trait]
/// impl Authenticator for JwtAuth {
///     type Identity = MyUser;
///
///     async fn authenticate(
///         &self,
///         headers: &HeaderMap,
///         body: Option<&Bytes>,
///     ) -> Result<MyUser, AuthError> {
///         let token = headers.get("Authorization")
///             .and_then(|v| v.to_str().ok())
///             .ok_or_else(|| AuthError::new("Missing token"))?;
///         decode_jwt(token).map_err(|e| AuthError::new(e.to_string()))
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Authenticator: Send + Sync + 'static {
    /// 认证产出的身份类型。
    type Identity: Clone + Send + Sync + 'static;

    /// 执行认证，返回身份或错误。
    ///
    /// `body` 需先注册 [`CachedBodyMiddleware`](crate::cached_body::CachedBodyMiddleware) 才可用，否则为 `None`。
    async fn authenticate(
        &self,
        headers: &HeaderMap,
        body: Option<&Bytes>,
    ) -> Result<Self::Identity, AuthError>;
}

/// 认证中间件，调用 [`Authenticator`] 验证请求。
///
/// 成功将 `Identity` 存入 request extensions，失败返回 `401`。
///
/// # Example
///
/// ```rust
/// // 全局认证
/// app.middleware(AuthMiddleware::new(JwtAuth));
///
/// // 排除公开路径
/// app.middleware_except("/login", AuthMiddleware::new(JwtAuth));
///
/// // 需读取 body 的认证（如 webhook 签名校验），须先注册 CachedBodyMiddleware
/// app.middleware(CachedBodyMiddleware);
/// app.middleware(AuthMiddleware::new(WebhookAuth));
/// ```
pub struct AuthMiddleware<A: Authenticator> {
    authenticator: A,
}

impl<A: Authenticator> AuthMiddleware<A> {
    pub fn new(authenticator: A) -> Self {
        Self { authenticator }
    }
}

#[async_trait::async_trait]
impl<A: Authenticator> Middleware for AuthMiddleware<A> {
    async fn handle(&self, mut req: Request, next: Next) -> Response {
        let cached_body = req.extensions().get::<CachedBody>().map(|c| &c.0);

        match self
            .authenticator
            .authenticate(req.headers(), cached_body)
            .await
        {
            Ok(identity) => {
                req.extensions_mut().insert(identity);
                next.run(req).await
            }
            Err(e) => {
                let body = serde_json::json!({
                    "code": 401,
                    "message": e.0,
                });
                (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
            }
        }
    }
}

/// 当前用户提取器，从 request extensions 获取已认证的身份。
///
/// `T` 须与 [`Authenticator::Identity`] 一致，未认证时返回 `401`。
///
/// # Example
///
/// ```rust
/// #[get("/me")]
/// async fn me(&self, CurrentUser(user): CurrentUser<MyUser>) -> webr::Json<MyUser> {
///     webr::Json(user)
/// }
/// ```
pub struct CurrentUser<T>(pub T);

impl<T, S> FromRequestParts<S> for CurrentUser<T>
where
    T: Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<T>()
            .cloned()
            .map(Self)
            .ok_or_else(|| Error::Http {
                status: StatusCode::UNAUTHORIZED,
                message: "Authentication required".into(),
            })
    }
}

/// 鉴权守卫 trait，实现权限检查逻辑。
///
/// `Ok(())` 放行，`Err(Error)` 拒绝（通常为 403）。
///
/// # Example
///
/// ```rust
/// struct AdminGuard;
///
/// #[async_trait::async_trait]
/// impl Guard for AdminGuard {
///     async fn check(&self, req: &Request) -> Result<(), Error> {
///         let user = req.extensions().get::<MyUser>()
///             .ok_or_else(|| Error::Http {
///                 status: StatusCode::UNAUTHORIZED,
///                 message: "Not authenticated".into(),
///             })?;
///         if user.role == "admin" {
///             Ok(())
///         } else {
///             Err(Error::Http {
///                 status: StatusCode::FORBIDDEN,
///                 message: "Admin access required".into(),
///             })
///         }
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Guard: Send + Sync + 'static {
    /// 检查请求权限，放行或拒绝。
    async fn check(&self, req: &Request) -> Result<(), Error>;
}

/// 鉴权中间件，通过 [`Guard`] 执行权限检查，失败返回 Guard 定义的错误。
///
/// 通常配合 `middleware_for` 按路径注册。
///
/// # Example
///
/// ```rust
/// app.middleware_for("/admin/**", GuardMiddleware::new(AdminGuard));
/// ```
pub struct GuardMiddleware {
    guard: Arc<dyn Guard>,
}

impl GuardMiddleware {
    pub fn new<G: Guard>(guard: G) -> Self {
        Self {
            guard: Arc::new(guard),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for GuardMiddleware {
    async fn handle(&self, req: Request, next: Next) -> Response {
        match self.guard.check(&req).await {
            Ok(()) => next.run(req).await,
            Err(e) => e.into_response(),
        }
    }
}
