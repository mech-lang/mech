#[cfg(feature = "cli_core")]
pub mod app;
#[cfg(feature = "bundle_web")]
pub mod bundle_web;
#[cfg(any(feature = "serve", feature = "run"))]
pub mod capabilities;
#[cfg(feature = "cli_core")]
pub mod commands;
#[cfg(any(feature = "serve", feature = "run"))]
pub mod config;
#[cfg(feature = "cli_core")]
pub(crate) mod diagnostics;
#[cfg(feature = "run")]
pub mod host_factories;
#[cfg(feature = "run")]
pub mod host_grants;
#[cfg(feature = "cli_core")]
pub(crate) mod outcome;
#[cfg(any(feature = "formatter", feature = "serve"))]
pub(crate) mod resources;
#[cfg(feature = "run")]
pub mod run;
#[cfg(feature = "run")]
pub mod run_options;
#[cfg(feature = "run")]
pub mod runtime_plan;
#[cfg(feature = "serve")]
pub mod serve_options;

#[cfg(all(test, any(feature = "serve", feature = "bundle_web", feature = "run")))]
pub(crate) static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
