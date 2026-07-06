//! Webr 中间件扩展。

pub mod auth;
pub mod cached_body;

pub use auth::{AuthError, AuthMiddleware, Authenticator, CurrentUser, Guard, GuardMiddleware};
pub use cached_body::{CachedBody, CachedBodyMiddleware};
pub use webr_web::middleware::{Middleware, Next};
