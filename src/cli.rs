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
#[cfg(any(feature = "build", feature = "test"))]
pub(crate) mod module_execution;
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

#[cfg(feature = "cli_core")]
pub(crate) fn rounds_per_step_value_parser() -> clap::builder::RangedU64ValueParser<usize> {
    clap::builder::RangedU64ValueParser::<usize>::new().range(1..)
}

#[cfg(all(test, any(
    feature = "serve",
    feature = "bundle_web",
    feature = "run",
    feature = "formatter",
    feature = "repl",
)))]
pub(crate) static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(test, any(
    feature = "serve",
    feature = "bundle_web",
    feature = "run",
    feature = "formatter",
    feature = "repl",
)))]
pub(crate) fn lock_current_dir() -> std::sync::MutexGuard<'static, ()> {
    CURRENT_DIR_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
