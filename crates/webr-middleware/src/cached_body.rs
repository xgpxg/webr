use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use webr_core::middleware::{Middleware, Next};

/// 缓存的请求体，由 [`CachedBodyMiddleware`] 写入 request extensions，
/// 供下游中间件读取。
#[derive(Clone)]
pub struct CachedBody(pub(crate) Bytes);

/// 请求体缓存中间件，将 body 读入内存并存入 request extensions，
/// 解决 body 只能消费一次的问题。
///
/// # Example
///
/// ```rust
/// app.middleware(CachedBodyMiddleware);
/// ```
///
/// # Note
///
/// 所有请求的 body 都会缓冲到内存，仅在需要重复读取时注册。
pub struct CachedBodyMiddleware;

#[async_trait::async_trait]
impl Middleware for CachedBodyMiddleware {
    async fn handle(&self, req: Request, next: Next) -> Response {
        let (parts, body) = req.into_parts();
        let bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(b) => b,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
            }
        };

        let mut parts = parts;
        parts.extensions.insert(CachedBody(bytes.clone()));
        let req = Request::from_parts(parts, Body::from(bytes));

        next.run(req).await
    }
}
