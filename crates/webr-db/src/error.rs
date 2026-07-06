use std::fmt;

/// Database error type wrapping sqlx errors and configuration issues.
#[derive(Debug)]
pub enum DbError {
    /// Wrapped sqlx error
    Sqlx(sqlx::Error),
    /// Connection pool is closed
    PoolClosed,
    /// Configuration error (missing fields, bad URL, etc.)
    Config(String),
    /// Insert operation did not return the created record
    InsertFailed,
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlx(e) => write!(f, "SQLx error: {e}"),
            Self::PoolClosed => write!(f, "Connection pool is closed"),
            Self::Config(msg) => write!(f, "Database config error: {msg}"),
            Self::InsertFailed => write!(f, "Insert failed: no record returned"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}

