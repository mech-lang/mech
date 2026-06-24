#[cfg(feature = "bundle_web")]
pub mod bundle_web;
#[cfg(feature = "serve")]
pub mod capabilities;
#[cfg(any(feature = "serve", feature = "run"))]
pub mod config;
#[cfg(feature = "run")]
pub mod run;

#[cfg(all(test, any(feature = "serve", feature = "bundle_web", feature = "run")))]
pub(crate) static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
