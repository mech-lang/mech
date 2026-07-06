#[cfg(any(
    feature = "build",
    feature = "formatter",
    feature = "run",
    feature = "serve",
    feature = "bundle_web",
    feature = "test",
    feature = "repl"
))]
pub mod app;
#[cfg(feature = "bundle_web")]
pub mod bundle_web;
#[cfg(any(feature = "serve", feature = "run"))]
pub mod capabilities;
pub mod commands;
#[cfg(any(feature = "serve", feature = "run"))]
pub mod config;
pub(crate) mod diagnostics;
#[cfg(feature = "run")]
pub mod host_factories;
pub(crate) mod paths;
pub(crate) mod resources;
#[cfg(feature = "run")]
pub mod run;
pub(crate) mod source_discovery;

#[cfg(all(test, any(feature = "serve", feature = "bundle_web", feature = "run")))]
pub(crate) static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
