pub mod app;
pub mod component;
pub mod error;
pub mod extract;
pub mod middleware;
pub mod response;
pub mod router;

// Re-export core types for convenience
pub use webr_core::config::{ConfigLoader, LogConfig, ServerConfig};
pub use webr_core::context::ApplicationContext;
pub use webr_core::error::FrameworkError;
pub use webr_core::inject::Inject;

/// Result type alias for the Webr web framework
pub type Result<T> = std::result::Result<T, error::Error>;
