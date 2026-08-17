#![allow(warnings)]

#[cfg(feature = "browser_host_dom")]
mod host;

#[cfg(feature = "browser_project_core")]
mod project;

#[cfg(feature = "browser_compute")]
mod gpu;

#[cfg(all(feature = "browser_compute", feature = "browser_project"))]
mod mixed_compute;

#[cfg(feature = "browser_project_core")]
pub use project::*;

#[cfg(all(feature = "browser_compute", feature = "browser_project"))]
pub use mixed_compute::*;
