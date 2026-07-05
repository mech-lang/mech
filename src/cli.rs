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
#[cfg(any(feature = "serve", feature = "run"))]
pub mod config;
#[cfg(feature = "run")]
pub mod host_factories;
#[cfg(feature = "run")]
pub mod run;

#[cfg(all(test, any(feature = "serve", feature = "bundle_web", feature = "run")))]
pub(crate) static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
