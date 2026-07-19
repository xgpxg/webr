use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use webr_core::error::FrameworkError;

/// User-facing error type for the Webr web framework.
///
/// This type is used in handler return types, extractors, and middleware.
/// It covers HTTP errors, database/cache errors,
/// and a catch-all internal variant.
///
/// Framework-internal errors (DI container lifecycle) are represented
/// by [`FrameworkError`] and converted to `Internal` at the application boundary.
#[derive(Debug)]
pub enum Error {
    /// HTTP business error with status code and message
    Http { status: StatusCode, message: String },
    /// Database error
    Database(Box<dyn std::error::Error + Send + Sync>),
    /// Cache error
    Cache(Box<dyn std::error::Error + Send + Sync>),
    /// Catch-all internal error
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { status, message } => write!(f, "HTTP {}: {message}", status.as_u16()),
            Self::Database(e) => write!(f, "Database: {e}"),
            Self::Cache(e) => write!(f, "Cache: {e}"),
            Self::Internal(msg) => write!(f, "Internal: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(e) | Self::Cache(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

/// Framework internal error → user-facing error (boundary conversion)
impl From<FrameworkError> for Error {
    fn from(e: FrameworkError) -> Self {
        Self::Internal(e.to_string())
    }
}

/// Validator validation errors → 422 Unprocessable Entity
#[cfg(feature = "validator")]
impl From<validator::ValidationErrors> for Error {
    fn from(e: validator::ValidationErrors) -> Self {
        Self::Http {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: e.to_string(),
        }
    }
}

// ─── Response conversion ──────────────────────────────────────────

/// Generic error response body
#[derive(Serialize)]
struct ErrorBody {
    code: u16,
    message: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::Http { status, message } => (status, message),
            Self::Database(e) => {
                tracing::error!("Database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            Self::Cache(e) => {
                tracing::error!("Cache error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = ErrorBody {
            code: status.as_u16(),
            message,
        };

        (status, axum::Json(body)).into_response()
    }
}
