#[cfg(feature = "serve")]
pub mod capabilities;
#[cfg(feature = "serve")]
pub mod config;

#[cfg(all(test, feature = "serve"))]
pub(crate) static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
