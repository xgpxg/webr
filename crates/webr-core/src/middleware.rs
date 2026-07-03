use std::borrow::Cow;
use std::sync::Arc;

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use futures_util::FutureExt;

/// 中间件 trait，用于请求拦截与处理。
///
/// 实现该 trait 可创建自定义中间件，通过 [`AppBuilder::middleware`] 注册。
///
/// # 自定义中间件
///
/// ```rust
/// use webr::middleware::{Middleware, Next};
/// use webr::async_trait::async_trait;
/// use axum::extract::Request;
/// use axum::response::Response;
///
/// struct AuthMiddleware;
///
/// #[async_trait]
/// impl Middleware for AuthMiddleware {
///     async fn handle(&self, request: Request, next: Next) -> Response {
///         // 前置处理：校验 Token
///         // let token = request.headers().get("Authorization");
///         // ...
///
///         // 传递给下一个中间件或 handler
///         let response = next.run(request).await;
///
///         // 后置处理：修改响应
///         response
///     }
/// }
/// ```
///
/// # 内置中间件
///
/// - [`LoggerMiddleware`] — 请求日志（方法、路径、状态码、耗时）
/// - [`CorsMiddleware`]   — CORS 跨域支持
/// - [`UnifiedResponse`]  — 统一 JSON 响应包装
/// - [`PanicRecovery`]    — Panic 恢复，防止 handler panic 导致进程崩溃
///
/// # 用法
///
/// ```rust
/// #[webr::main]
/// async fn main(app: &mut webr::AppBuilder) -> Result<(), Error> {
///     // 全局中间件：对所有路由生效
///     app.middleware(LoggerMiddleware);
///     app.middleware(PanicRecovery);
///
///     // CORS 跨域（Builder 模式配置）
///     app.middleware(
///         CorsMiddleware::new()
///             .allow_origin("*")
///             .allow_methods(["GET", "POST", "PUT", "DELETE"])
///             .allow_headers(["Content-Type", "Authorization"])
///             .max_age(3600),
///     );
///
///     // 统一响应包装：2xx JSON → {"code": 200, "message": "success", "data": ...}
///     app.unified_response();
///
///     // 局部中间件：仅对匹配路径生效
///     app.middleware_for("/api/**", LoggerMiddleware);
///
///     // 排除模式：对匹配路径以外的生效
///     app.middleware_except("/health", LoggerMiddleware);
///
///     Ok(())
/// }
/// ```
///
/// # 执行顺序
///
/// 中间件按注册顺序依次执行，链式调用：
/// 
/// `Middleware1 → Middleware2 → ... → Handler → ... → Middleware2 → Middleware1`
#[async_trait::async_trait]
pub trait Middleware: Send + Sync + 'static {
    /// 处理请求，实现自定义中间件逻辑。
    ///
    /// 调用 `next.run(request).await` 将请求传递给链中的下一个中间件或 handler。
    async fn handle(&self, request: Request, next: Next) -> Response;
}

/// 中间件链中的下一个处理器。
///
/// 封装 axum 的 `Next`，在 [`Middleware::handle`] 中通过 `next.run(request).await`
/// 将请求传递给链中的下一个中间件或最终 handler。
pub struct Next {
    inner: axum::middleware::Next,
}

impl Next {
    pub fn new(inner: axum::middleware::Next) -> Self {
        Self { inner }
    }

    /// 将请求传递给下一个处理器，返回响应。
    pub async fn run(self, request: Request) -> Response {
        self.inner.run(request).await
    }
}

/// 局部范围中间件：只对匹配路径模式的路由生效。
///
/// 支持两种模式：
/// - **包含模式**：只对匹配路径执行中间件
/// - **排除模式**：只对不匹配路径执行中间件
///
/// 路径模式语法：
/// - `/api/**` — 前缀匹配，所有 `/api/` 开头的路径
/// - `/health` — 精确匹配
///
/// 通过 [`AppBuilder::middleware_for`] 和 [`AppBuilder::middleware_except`] 注册。
pub struct ScopedMiddleware {
    pattern: &'static str,
    prefix: Cow<'static, str>,
    exact: bool,
    middleware: Arc<dyn Middleware>,
    exclude: bool,
    name: &'static str,
}

impl ScopedMiddleware {
    /// 创建包含模式：只对匹配的路径执行中间件
    pub fn new<M: Middleware>(pattern: &'static str, middleware: Arc<M>) -> Self {
        let (prefix, exact) = parse_pattern(pattern);
        Self {
            pattern,
            prefix,
            exact,
            middleware,
            exclude: false,
            name: short_type_name::<M>(),
        }
    }

    /// 创建排除模式：只对 **不匹配** 的路径执行中间件
    pub fn exclude<M: Middleware>(pattern: &'static str, middleware: Arc<M>) -> Self {
        let (prefix, exact) = parse_pattern(pattern);
        Self {
            pattern,
            prefix,
            exact,
            middleware,
            exclude: true,
            name: short_type_name::<M>(),
        }
    }

    /// 判断请求路径是否匹配（仅按前缀匹配，暂时没必要引入正则吧）
    pub fn matches(&self, path: &str) -> bool {
        let hit = if self.exact {
            path == self.prefix
        } else {
            path.starts_with(self.prefix.as_ref())
        };
        if self.exclude {
            !hit
        } else {
            hit
        }
    }

    pub fn pattern(&self) -> &'static str {
        self.pattern
    }

    pub fn middleware(&self) -> &Arc<dyn Middleware> {
        &self.middleware
    }

    /// 返回中间件类型名称（用于启动日志）
    pub fn middleware_name(&self) -> &'static str {
        self.name
    }
}

/// 解析路径模式：
/// - `/api/**` → prefix=`/api/`, exact=false
/// - `/health` → prefix=`/health`, exact=true
fn parse_pattern(pattern: &'static str) -> (Cow<'static, str>, bool) {
    if let Some(prefix) = pattern.strip_suffix("/**") {
        if prefix.ends_with('/') {
            (Cow::Borrowed(prefix), false)
        } else {
            (Cow::Owned(format!("{prefix}/")), false)
        }
    } else {
        (Cow::Borrowed(pattern), true)
    }
}

/// 提取类型的简短名称（去掉模块路径前缀）
fn short_type_name<T>() -> &'static str {
    let full = std::any::type_name::<T>();
    full.rsplit_once("::").map(|(_, name)| name).unwrap_or(full)
}

///////////////////////// 以下为内置中间件 /////////////////////////

/// 请求日志中间件，记录每个请求的方法、路径、状态码和耗时。
///
/// # 用法
///
/// ```rust
/// // 全局启用
/// app.middleware(LoggerMiddleware);
///
/// // 仅对特定路径启用
/// app.middleware_for("/api/**", LoggerMiddleware);
/// ```
///
/// # 日志输出示例
///
/// ```text
/// -> GET /api/users
/// <- GET /api/users 200 OK (1.23ms)
/// ```
pub struct LoggerMiddleware;

#[async_trait::async_trait]
impl Middleware for LoggerMiddleware {
    async fn handle(&self, request: Request, next: Next) -> Response {
        // Method 是 Copy 类型（内部枚举），Uri 需 clone（堆分配不可避免，因 request 被 move）
        let method = request.method().clone();
        let uri = request.uri().clone();
        let start = std::time::Instant::now();

        tracing::info!("-> {method} {uri}");

        let response = next.run(request).await;

        let elapsed = start.elapsed();
        let status = response.status();
        tracing::info!("<- {method} {uri} {status} ({elapsed:?})");

        response
    }
}

// ─── CORS 跨域中间件 ─────────────────────────────────────

/// CORS 跨域中间件，通过 Builder 模式配置跨域策略。
///
/// 自动处理 `OPTIONS` 预检请求并返回 `204 No Content`，
/// 正常请求则追加 CORS 响应头。
///
/// # 默认配置
///
/// | 配置项 | 默认值 |
/// |--------|--------|
/// | allow_origin | `*` |
/// | allow_methods | `GET,POST,PUT,DELETE,PATCH,OPTIONS` |
/// | allow_headers | `Content-Type,Authorization` |
/// | allow_credentials | `false` |
/// | max_age | 无 |
///
/// # 用法
///
/// ## 基础用法：允许所有来源
///
/// ```rust
/// app.middleware(CorsMiddleware::new());
/// ```
///
/// ## 自定义配置
///
/// ```rust
/// app.middleware(
///     CorsMiddleware::new()
///         .allow_origin("https://example.com")
///         .allow_methods(["GET", "POST", "PUT", "DELETE"])
///         .allow_headers(["Content-Type", "Authorization"])
///         .max_age(3600),
/// );
/// ```
///
/// ## 携带 Cookie（需指定具体来源）
///
/// ```rust
/// app.middleware(
///     CorsMiddleware::new()
///         .allow_origin("https://example.com")  // 不能使用 "*"
///         .allow_credentials(true)
///         .allow_headers(["Content-Type", "X-Token"]),
/// );
/// ```
///
/// # 注意事项
///
/// - `allow_credentials(true)` 与 `allow_origin("*")` 互斥，同时设置会 panic
/// - credentials 模式下会自动添加 `Vary: Origin` 头，防止 CDN/代理缓存错误响应
pub struct CorsMiddleware {
    allow_origin: HeaderValue,
    allow_methods: HeaderValue,
    allow_headers: HeaderValue,
    allow_credentials: bool,
    max_age: Option<HeaderValue>,
}

impl CorsMiddleware {
    pub fn new() -> Self {
        Self {
            allow_origin: HeaderValue::from_static("*"),
            allow_methods: HeaderValue::from_static("GET,POST,PUT,DELETE,PATCH,OPTIONS"),
            allow_headers: HeaderValue::from_static("Content-Type,Authorization"),
            allow_credentials: false,
            max_age: None,
        }
    }

    /// 允许的来源，默认 `"*"`。
    ///
    /// # Panics
    /// 当 `allow_credentials(true)` 已设置时，不允许使用 `"*"` 通配符。
    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        let s = origin.into();
        if s == "*" && self.allow_credentials {
            panic!("CORS: allow_origin(\"*\") is incompatible with allow_credentials(true). Set a specific origin.");
        }
        self.allow_origin = s
            .parse()
            .unwrap_or_else(|e| panic!("Invalid CORS allow_origin '{s}': {e}"));
        self
    }

    /// 允许的 HTTP 方法列表
    pub fn allow_methods(mut self, methods: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let s = Self::join(methods);
        self.allow_methods = s
            .parse()
            .unwrap_or_else(|e| panic!("Invalid CORS allow_methods '{s}': {e}"));
        self
    }

    /// 允许的请求头列表
    pub fn allow_headers(mut self, headers: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let s = Self::join(headers);
        self.allow_headers = s
            .parse()
            .unwrap_or_else(|e| panic!("Invalid CORS allow_headers '{s}': {e}"));
        self
    }

    /// 是否允许携带 Cookie。
    ///
    /// # Panics
    /// 当 `allow_origin("*")` 且 `allow(true)` 时 panic，
    /// 因为 CORS 规范禁止通配符来源与 credentials 同时使用。
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        if allow && self.allow_origin.as_bytes() == b"*" {
            panic!("CORS: allow_credentials(true) is incompatible with allow_origin(\"*\"). Set a specific origin first.");
        }
        self.allow_credentials = allow;
        self
    }

    /// 预检请求缓存时间（秒）
    pub fn max_age(mut self, seconds: u64) -> Self {
        self.max_age = Some(HeaderValue::from(seconds));
        self
    }

    fn join(items: impl IntoIterator<Item = impl AsRef<str>>) -> String {
        items
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn apply_cors_headers(&self, headers: &mut axum::http::HeaderMap, origin: Option<&str>) {
        use axum::http::header;

        // credentials 模式下不能用 "*"，回显请求 Origin
        let is_dynamic_origin = self.allow_origin.as_bytes() != b"*" || self.allow_credentials;
        let origin_value = if !is_dynamic_origin {
            HeaderValue::from_static("*")
        } else if let Some(o) = origin {
            HeaderValue::from_str(o).unwrap_or_else(|_| HeaderValue::from_static(""))
        } else {
            self.allow_origin.clone()
        };

        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_value);
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            self.allow_methods.clone(),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            self.allow_headers.clone(),
        );

        // 动态回显 Origin 时必须带 Vary: Origin，防止 CDN/代理缓存错误响应
        if is_dynamic_origin {
            headers.append(header::VARY, HeaderValue::from_static("Origin"));
        }

        if self.allow_credentials {
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
        }
        if let Some(ref max_age) = self.max_age {
            headers.insert(header::ACCESS_CONTROL_MAX_AGE, max_age.clone());
        }
    }
}

impl Default for CorsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for CorsMiddleware {
    async fn handle(&self, request: Request, next: Next) -> Response {
        let origin = request
            .headers()
            .get(axum::http::header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // OPTIONS 预检请求：直接返回 204
        if request.method() == axum::http::Method::OPTIONS {
            let mut resp = Response::builder()
                .status(axum::http::StatusCode::NO_CONTENT)
                .body(axum::body::Body::empty())
                .unwrap();
            self.apply_cors_headers(resp.headers_mut(), origin.as_deref());
            return resp;
        }

        // 正常请求：追加 CORS 头
        let mut response = next.run(request).await;
        self.apply_cors_headers(response.headers_mut(), origin.as_deref());
        response
    }
}

// ─── 统一响应中间件 ────────────────────────────

/// 统一响应包装中间件，将 2xx JSON 响应包装为标准格式。
///
/// # 包装格式
///
/// ```json
/// {
///     "code": 200,
///     "message": "success",
///     "data": { ... }  // 原始响应数据
/// }
/// ```
///
/// # 处理规则
///
/// | 响应类型 | 处理方式 |
/// |----------|----------|
/// | 2xx + JSON | 包装为标准格式 |
/// | 非 2xx（错误） | 原样透传 |
/// | 非 JSON 响应 | 原样透传 |
///
/// # 用法
///
/// ```rust
/// // 推荐使用便捷方法
/// app.unified_response();
///
/// // 或显式添加中间件
/// app.middleware(UnifiedResponse);
/// ```
///
/// # 示例
///
/// 原始响应：
/// ```json
/// {"name": "Alice", "age": 30}
/// ```
///
/// 包装后：
/// ```json
/// {"code": 200, "message": "success", "data": {"name": "Alice", "age": 30}}
/// ```
pub struct UnifiedResponse;

#[async_trait::async_trait]
impl Middleware for UnifiedResponse {
    async fn handle(&self, request: Request, next: Next) -> Response {
        let response = next.run(request).await;
        let status = response.status();

        // 仅包装成功（2xx）响应
        if !status.is_success() {
            return response;
        }

        // 仅包装 JSON 响应
        let is_json = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .is_some_and(|v| v.as_bytes().starts_with(b"application/json"));

        if !is_json {
            return response;
        }

        // 读取响应体
        let (parts, body) = response.into_parts();
        let bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(b) => b,
            Err(_) => {
                return Response::from_parts(parts, axum::body::Body::empty());
            }
        };

        // 解析 JSON 并包装
        match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(data) => {
                let wrapped = serde_json::json!({
                    "code": status.as_u16(),
                    "message": "success",
                    "data": data
                });
                (status, axum::Json(wrapped)).into_response()
            }
            Err(_) => {
                // 非合法 JSON，原样透传
                Response::from_parts(parts, axum::body::Body::from(bytes))
            }
        }
    }
}

/// Panic 恢复中间件，捕获 handler 中的 panic 并返回 500 错误。
///
/// 防止因未处理的 panic 导致进程崩溃，提高服务稳定性。
///
/// # 用法
///
/// ```rust
/// // 建议作为第一个全局中间件，确保捕获所有 panic
/// app.middleware(PanicRecovery);
/// ```
///
/// # 行为
///
/// - 捕获 handler 内的 panic
/// - 记录 panic 信息到日志（`tracing::error!`）
/// - 返回 `500 Internal Server Error` 响应
/// - 进程继续运行，不会崩溃
///
/// # 示例
///
/// ```rust
/// #[get("/panic")]
/// async fn trigger_panic(&self) -> &'static str {
///     panic!("Something went wrong!");  // 不会导致进程崩溃
/// }
/// // 响应: 500 Internal Server Error
/// // 日志: Panic in handler GET /panic Something went wrong!
/// ```
pub struct PanicRecovery;

#[async_trait::async_trait]
impl Middleware for PanicRecovery {
    async fn handle(&self, request: Request, next: Next) -> Response {
        let method = request.method().clone();
        let uri = request.uri().clone();

        // catch_unwind 捕获 handler 内的 panic，防止进程崩溃
        let result = std::panic::AssertUnwindSafe(next.run(request))
            .catch_unwind()
            .await;

        match result {
            Ok(response) => response,
            Err(panic_payload) => {
                let msg = extract_panic_message(&*panic_payload);
                tracing::error!("Panic in handler {} {}: {}", method, uri, msg);
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
            }
        }
    }
}

/// 从 panic payload 提取可读消息
fn extract_panic_message(payload: &dyn std::any::Any) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    if let Some(s) = payload.downcast_ref::<&str>() {
        return s.to_string();
    }
    if let Some(s) = payload.downcast_ref::<std::fmt::Arguments<'_>>() {
        return s.to_string();
    }
    "Internal server error (panic)".to_string()
}
