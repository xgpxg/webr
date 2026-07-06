//! Web layer component registration: `ConfigEntry`, `ComponentEntry`, `MountFn`, `RouteDescriptor`.
//!
//! Core `Component` trait and `ComponentRegistration` are defined in `webr-core`.

use webr_core::context::ApplicationContext;
use webr_core::error::FrameworkError;

use crate::error::Error;

pub use webr_core::component::{Component, ComponentRegistration};

/// Config entry: submitted via `inventory::submit!` by the `#[config]` macro,
/// collected at startup by `AppBuilder::build()` and registered into the IoC container.
pub struct ConfigEntry {
    /// Parse a TOML root node and register the config type into the IoC container
    pub register: fn(&toml::Value, &mut ApplicationContext<Error>) -> Result<(), FrameworkError>,
}

inventory::collect!(ConfigEntry);

/// Route mount function type
pub type MountFn =
    fn(&ApplicationContext<Error>, &mut crate::router::WebrRouter) -> Result<(), Error>;

/// Route descriptor: (HTTP method, path, controller name)
pub type RouteDescriptor = (&'static str, &'static str, &'static str);

/// Auto-registered component entry, submitted by #[controller] / #[component] macros
/// via `inventory::submit!`. Collected at startup by `inventory::iter::<ComponentEntry>()`.
pub struct ComponentEntry {
    /// Register the component into the IoC container
    pub register: fn(&mut ApplicationContext<Error>),
    /// Mount controller routes onto the router (controllers only)
    pub mount: Option<MountFn>,
    /// Route metadata for startup route table printing
    pub routes: &'static [RouteDescriptor],
}

inventory::collect!(ComponentEntry);
