#[cfg(feature = "cli_core")]
pub mod app;
#[cfg(feature = "bundle_web")]
pub mod bundle_web;
#[cfg(any(feature = "serve", feature = "run"))]
pub mod capabilities;
#[cfg(feature = "cli_core")]
pub mod commands;
#[cfg(feature = "compute_backends_native")]
pub(crate) mod compute;
#[cfg(any(feature = "build", feature = "serve", feature = "run"))]
pub mod config;
#[cfg(any(feature = "build", feature = "run"))]
pub(crate) mod host_configuration;
#[cfg(feature = "run")]
pub mod host_factories;
#[cfg(any(feature = "build", feature = "run"))]
pub mod host_grants;
#[cfg(feature = "build")]
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

/// Stable compute selectors accepted by shipping CLI products.
///
/// Experimental backend IDs remain available to library and benchmark callers,
/// but `run` and `serve` share this product-facing admission policy.
#[cfg(any(feature = "run", feature = "serve"))]
pub(crate) const STABLE_COMPUTE_BACKEND_SELECTORS: [&str; 5] =
    ["auto", "cpu", "gpu", "cpu-scalar", "wgpu"];

/// Native `run` also exposes the fixed-shape SIMD and Cranelift executors.
/// Browser serving keeps the stable selector list because those executors are
/// not available in the browser product.
#[cfg(feature = "run")]
pub(crate) const NATIVE_COMPUTE_BACKEND_SELECTORS: [&str; 7] = [
    "auto",
    "cpu",
    "gpu",
    "cpu-scalar",
    "cpu-simd",
    "cpu-jit",
    "wgpu",
];

#[cfg(all(
    test,
    any(
        feature = "serve",
        feature = "bundle_web",
        feature = "build",
        feature = "run",
        feature = "formatter",
    )
))]
pub(crate) static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(
    test,
    any(
        feature = "serve",
        feature = "bundle_web",
        feature = "build",
        feature = "run",
        feature = "formatter",
    )
))]
pub(crate) fn lock_current_dir() -> std::sync::MutexGuard<'static, ()> {
    CURRENT_DIR_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
