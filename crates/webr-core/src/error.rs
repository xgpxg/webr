use std::fmt;

/// Framework internal error type.
///
/// Covers DI container lifecycle errors: component registration, resolution,
/// dependency graph analysis, and configuration loading.
///
/// These errors occur during application startup (build phase) and are
/// typically converted to `webr_web::error::Error::Internal` at the
/// application boundary.
#[derive(Debug)]
pub enum FrameworkError {
    /// Component already registered
    DuplicateComponent(&'static str),
    /// Component not found in container
    ComponentNotFound(&'static str),
    /// Type downcast failed
    DowncastFailed(&'static str),
    /// Circular dependency detected
    CircularDependency(String),
    /// Component dependency not registered
    DependencyNotFound(String),
    /// Configuration loading or parsing error
    ConfigError(String),
}

impl fmt::Display for FrameworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateComponent(name) => write!(f, "Component '{name}' already registered"),
            Self::ComponentNotFound(name) => write!(f, "Component '{name}' not found"),
            Self::DowncastFailed(name) => write!(f, "Failed to downcast '{name}'"),
            Self::CircularDependency(msg) => write!(f, "Circular dependency: {msg}"),
            Self::DependencyNotFound(msg) => write!(f, "Dependency not found: {msg}"),
            Self::ConfigError(msg) => write!(f, "Config: {msg}"),
        }
    }
}

impl std::error::Error for FrameworkError {}
