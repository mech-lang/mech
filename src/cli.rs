#[cfg(feature = "bundle_web")]
pub mod bundle_web;
#[cfg(feature = "serve")]
pub mod capabilities;
#[cfg(feature = "serve")]
pub mod config;

#[cfg(all(test, any(feature = "serve", feature = "bundle_web")))]
pub(crate) static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
