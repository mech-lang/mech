#![allow(warnings)]

#[cfg(all(
    feature = "browser_project_core",
    not(feature = "browser_host_console")
))]
compile_error!(
    "browser_project_core must include browser_host_console because every WasmDocument controller exports WasmRepl"
);

#[cfg(all(feature = "browser_project", not(feature = "state_machines")))]
compile_error!(
    "browser_project must include state_machines because served documents may contain FSM specifications and implementations"
);

mod repl;

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
pub use repl::*;
