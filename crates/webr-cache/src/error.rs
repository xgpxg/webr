use std::fmt;

/// Cache error type.
#[derive(Debug)]
pub enum CacheError {
    Serialization(String),
    Backend(String),
    Config(String),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization(msg) => write!(f, "cache serialization error: {msg}"),
            Self::Backend(msg) => write!(f, "cache backend error: {msg}"),
            Self::Config(msg) => write!(f, "cache config error: {msg}"),
        }
    }
}

impl std::error::Error for CacheError {}

impl From<serde_json::Error> for CacheError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

#[cfg(feature = "redis-backend")]
impl From<redis::RedisError> for CacheError {
    fn from(e: redis::RedisError) -> Self {
        Self::Backend(e.to_string())
    }
}

#[cfg(feature = "sled-backend")]
impl From<sled::Error> for CacheError {
    fn from(e: sled::Error) -> Self {
        Self::Backend(e.to_string())
    }
}
