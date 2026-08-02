//! Web layer controller registration
use webr_core::context::ApplicationContext;
use webr_core::error::FrameworkError;

use crate::error::Error;

pub use webr_core::component::{Component, ComponentRegistration};

/// Route mount function type
pub type MountFn =
    fn(&ApplicationContext<FrameworkError>, &mut crate::router::WebrRouter) -> Result<(), Error>;

/// Route descriptor: (HTTP method, path, controller name)
pub type RouteDescriptor = (&'static str, &'static str, &'static str);

/// Controller entry for route mounting, submitted by `#[controller]` macro
/// via `inventory::submit!`. Collected at startup by `AppBuilder::build()`.
pub struct ControllerEntry {
    /// Mount controller routes onto the router
    pub mount: Option<MountFn>,
    /// Route metadata for startup route table printing
    pub routes: &'static [RouteDescriptor],
}

inventory::collect!(ControllerEntry);
