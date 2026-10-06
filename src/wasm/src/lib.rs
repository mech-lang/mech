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

#[cfg(feature = "browser_project_core")]
mod canonical_document;

#[cfg(feature = "browser_host_dom")]
mod host;

#[cfg(feature = "browser_project_core")]
mod project;

#[cfg(feature = "browser_compute")]
mod gpu;

#[cfg(feature = "browser_compute")]
mod mixed_compute;

#[cfg(feature = "browser_project_core")]
pub use canonical_document::*;
#[cfg(feature = "browser_project_core")]
pub use project::*;

#[cfg(feature = "browser_compute")]
pub use mixed_compute::*;
pub use repl::*;

#[cfg(feature = "syntax_inspection")]
mod inspection;
#[cfg(feature = "syntax_inspection")]
pub use inspection::*;

#[cfg(feature = "type_inspection")]
mod type_inspection;
#[cfg(feature = "type_inspection")]
pub use type_inspection::*;

#[cfg(feature = "i64_publication")]
mod i64_publication;
#[cfg(feature = "i64_publication")]
pub use i64_publication::*;
