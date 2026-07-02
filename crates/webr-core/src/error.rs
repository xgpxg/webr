use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

/// WebR 框架错误类型
#[derive(Debug)]
pub enum WebrError {
    /// 组件重复注册
    DuplicateComponent(&'static str),
    /// 组件未找到
    ComponentNotFound(&'static str),
    /// 类型转换失败
    DowncastFailed(&'static str),
    /// 循环依赖
    CircularDependency(String),
    /// HTTP 业务错误
    Http { status: StatusCode, message: String },
    /// 内部错误
    Internal(String),
    /// 配置错误
    ConfigError(String),
    /// 参数校验失败（422）
    Validation(Vec<ValidationFieldError>),
}

/// 校验失败的字段级错误详情
#[derive(Debug, Serialize)]
pub struct ValidationFieldError {
    /// 校验失败的字段名
    pub field: String,
    /// 错误描述
    pub message: String,
}

impl std::fmt::Display for WebrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateComponent(name) => write!(f, "Component '{name}' already registered"),
            Self::ComponentNotFound(name) => write!(f, "Component '{name}' not found"),
            Self::DowncastFailed(name) => write!(f, "Failed to downcast '{name}'"),
            Self::CircularDependency(msg) => write!(f, "Circular dependency: {msg}"),
            Self::Http { status, message } => write!(f, "HTTP {}: {message}", status.as_u16()),
            Self::Internal(msg) => write!(f, "Internal: {msg}"),
            Self::ConfigError(msg) => write!(f, "Config: {msg}"),
            Self::Validation(errors) => {
                write!(f, "Validation failed: ")?;
                for e in errors {
                    write!(f, "[{}: {}] ", e.field, e.message)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for WebrError {}

/// 通用错误响应体
#[derive(Serialize)]
struct ErrorBody {
    /// HTTP 状态码
    code: u16,
    /// 错误描述
    message: String,
}

/// 校验错误响应体（422）
#[derive(Serialize)]
struct ValidationBody {
    /// 固定为 422
    code: u16,
    /// 固定为 "Validation failed"
    message: &'static str,
    /// 字段级错误列表
    errors: Vec<ValidationFieldError>,
}

/// 实现 `IntoResponse`，使 `Result<T, WebrError>` 可直接作为 axum handler 返回值
impl IntoResponse for WebrError {
    fn into_response(self) -> axum::response::Response {
        // 校验错误：422 + 字段错误数组
        if let Self::Validation(errors) = self {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                axum::Json(ValidationBody {
                    code: 422,
                    message: "Validation failed",
                    errors,
                }),
            )
                .into_response();
        }

        let (status, message) = match self {
            Self::Http { status, message } => (status, message),
            Self::ComponentNotFound(msg) => (
                StatusCode::NOT_FOUND,
                format!("Component '{msg}' not found"),
            ),
            Self::DuplicateComponent(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Component '{msg}' already registered"),
            ),
            Self::DowncastFailed(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to downcast '{msg}'"),
            ),
            Self::CircularDependency(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Circular dependency: {msg}"),
            ),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Config: {msg}")),
            Self::Validation(_) => unreachable!(),
        };

        let body = ErrorBody {
            code: status.as_u16(),
            message,
        };

        (status, axum::Json(body)).into_response()
    }
}

/// WebR 统一 Result 别名
pub type WebrResult<T> = Result<T, WebrError>;

/// 将 `validator::ValidationErrors` 转换为 `WebrError::Validation`
impl From<validator::ValidationErrors> for WebrError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let field_errors = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errs)| {
                errs.iter().map(move |e| ValidationFieldError {
                    field: (*field).to_string(),
                    message: e
                        .message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| e.code.to_string()),
                })
            })
            .collect();
        Self::Validation(field_errors)
    }
}
