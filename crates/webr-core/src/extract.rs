use crate::error::Error;
use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use validator::Validate;

/// 提取并校验：将 axum 提取错误转为 `Error::Http`，然后执行 `Validate`
fn extract_and_validate<T: Validate>(
    value: Result<T, impl std::fmt::Display>,
    kind: &'static str,
) -> Result<T, Error> {
    let value = value.map_err(|e| Error::Http {
        status: StatusCode::BAD_REQUEST,
        message: format!("Invalid {kind}: {e}"),
    })?;
    value.validate()?;
    Ok(value)
}

/// 路径参数提取器，对应 `#[get("/users/{id}")]` 中的 `{id}`。
///
/// # Example
/// ```rust
/// #[get("/users/{id}")]
/// async fn get_user(&self, webr::Path(id): webr::Path<i64>) -> webr::Json<User> {
///     // ...
/// }
///
/// // 多参数
/// #[get("/users/{user_id}/posts/{post_id}")]
/// async fn get_post(&self, webr::Path((user_id, post_id)): webr::Path<(i64, i64)>) -> webr::Json<Post> {
///     // ...
/// }
/// ```
pub struct Path<T>(pub T);

impl<T, S> axum::extract::FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum_path = axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error::Http {
                status: StatusCode::BAD_REQUEST,
                message: format!("Invalid path parameter: {e}"),
            })?;
        Ok(Self(axum_path.0))
    }
}

/// 查询参数提取器，对应 `?page=1&size=10`，自动校验。
///
/// # Example
/// ```rust
/// #[derive(Deserialize, Validate)]
/// struct PageQuery {
///     #[validate(range(min = 1))]
///     page: u32,
///     #[validate(range(min = 1, max = 100))]
///     size: u32,
/// }
///
/// #[get("/items")]
/// async fn list(&self, webr::Query(q): webr::Query<PageQuery>) -> webr::Json<Vec<Item>> {
///     // q.page, q.size 已校验通过
///     // ...
/// }
/// ```
pub struct Query<T>(pub T);

impl<T, S> axum::extract::FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        extract_and_validate(
            axum::extract::Query::<T>::from_request_parts(parts, state)
                .await
                .map(|q| q.0),
            "query parameter",
        )
        .map(Self)
    }
}

/// JSON 请求体提取器，自动反序列化并校验。
///
/// # Example
/// ```rust
/// #[derive(Deserialize, Validate)]
/// struct CreateUser {
///     #[validate(length(min = 1, max = 50))]
///     name: String,
///     #[validate(email)]
///     email: String,
/// }
///
/// #[post("/users")]
/// async fn create(&self, webr::Json(body): webr::Json<CreateUser>) -> webr::Json<User> {
///     // body 已反序列化并校验通过
///     // ...
/// }
/// ```
pub struct Json<T>(pub T);

impl<T, S> axum::extract::FromRequest<S> for Json<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        extract_and_validate(
            axum::extract::Json::<T>::from_request(req, state)
                .await
                .map(|j| j.0),
            "request body",
        )
        .map(Self)
    }
}

/// 文件上传提取器，重新导出 axum 的 `Multipart`。
///
/// # Example
/// ```rust
/// #[post("/upload")]
/// async fn upload(&self, mut multipart: webr::Multipart) -> webr::Json<serde_json::Value> {
///     while let Some(field) = multipart.next_field().await.unwrap() {
///         let name = field.file_name().unwrap_or("unknown").to_string();
///         let data = field.bytes().await.unwrap();
///         // 处理文件...
///     }
///     webr::Json(serde_json::json!({ "uploaded": true }))
/// }
/// ```
pub use axum::extract::Multipart;

impl<T: serde::Serialize> axum::response::IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

/// 请求头提取器，将 header key 自动转为 snake_case 匹配字段名。
///
/// 无法自动匹配时使用 `#[serde(rename = "...")]` 指定。
///
/// # Example
/// ```rust
/// #[derive(Deserialize)]
/// struct AuthHeaders {
///     authorization: String,
///     #[serde(rename = "x-request-id")]
///     request_id: Option<String>,
/// }
///
/// #[get("/me")]
/// async fn me(&self, webr::Header(h): webr::Header<AuthHeaders>) -> webr::Json<User> {
///     // h.authorization, h.request_id
///     // ...
/// }
/// ```
pub struct Header<T>(pub T);

impl<T, S> axum::extract::FromRequestParts<S> for Header<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let map: HashMap<String, String> = parts
            .headers
            .iter()
            .map(|(k, v)| {
                (
                    to_snake_case(k.as_str()),
                    v.to_str().unwrap_or("").to_string(),
                )
            })
            .collect();

        let deserializer = serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(
            map.into_iter(),
        );
        let value = T::deserialize(deserializer).map_err(|e| Error::Http {
            status: StatusCode::BAD_REQUEST,
            message: format!("Invalid header: {e}"),
        })?;
        Ok(Self(value))
    }
}

/// Header key 转 snake_case：`-` → `_`，全部小写。
fn to_snake_case(s: &str) -> String {
    s.replace('-', "_").to_lowercase()
}

/// `HeaderMap` 扩展 trait，提供便捷的 header 取值与解析方法。
///
/// # Example
/// ```rust
/// use webr::HeaderMapExt;
///
/// async fn handler(headers: webr::HeaderMap) -> String {
///     let token = headers.get_str("authorization").unwrap_or("");
///     let page: i32 = headers.get_parsed("x-page").unwrap_or(1);
///     format!("token: {token}, page: {page}")
/// }
/// ```
pub trait HeaderMapExt {
    /// 获取 header 的字符串值，header 不存在或值非 UTF-8 时返回 `None`
    fn get_str(&self, name: &str) -> Option<&str>;
    /// 获取 header 并解析为目标类型，解析失败时返回 `None`
    fn get_parsed<T: std::str::FromStr>(&self, name: &str) -> Option<T>;
}

impl HeaderMapExt for axum::http::HeaderMap {
    fn get_str(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|v| v.to_str().ok())
    }

    fn get_parsed<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.get_str(name).and_then(|v| v.parse().ok())
    }
}

/// 表单提取器，解析 `application/x-www-form-urlencoded` 请求体，自动校验。
///
/// # Example
/// ```rust
/// #[derive(Deserialize, Validate)]
/// struct LoginForm {
///     #[validate(length(min = 1))]
///     username: String,
///     #[validate(length(min = 6))]
///     password: String,
/// }
///
/// #[post("/login")]
/// async fn login(&self, webr::Form(form): webr::Form<LoginForm>) -> webr::Json<Token> {
///     // form 已反序列化并校验通过
///     // ...
/// }
/// ```
pub struct Form<T>(pub T);

impl<T, S> axum::extract::FromRequest<S> for Form<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        extract_and_validate(
            axum::Form::<T>::from_request(req, state).await.map(|f| f.0),
            "form data",
        )
        .map(Self)
    }
}
