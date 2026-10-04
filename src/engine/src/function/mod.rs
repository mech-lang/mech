//! Function catalogs, environments, ABI modules, and canonical specialization.

#[cfg(all(
    feature = "dynamic-modules",
    any(feature = "semantic-compiler", feature = "resident-artifact")
))]
mod dynamic;
#[path = "catalog.rs"]
pub(crate) mod engine_catalog;
pub mod environment;
pub mod extensions;
#[cfg(feature = "program")]
pub mod external;
#[cfg(all(
    feature = "dynamic-modules",
    any(feature = "semantic-compiler", feature = "resident-artifact")
))]
pub use dynamic::DynamicModuleLoader;
#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
pub(crate) use dynamic::bind_dynamic_resident_operation;
#[cfg(all(feature = "semantic-compiler", feature = "functions"))]
pub mod module;
#[cfg(all(feature = "semantic-compiler", feature = "native"))]
pub mod native;
pub mod resolver;

pub use engine_catalog::*;
pub use environment::*;
pub use extensions::*;
#[cfg(feature = "program")]
pub use external::*;
#[cfg(all(feature = "semantic-compiler", feature = "functions"))]
pub use module::*;
#[cfg(all(feature = "semantic-compiler", feature = "native"))]
pub use native::*;
pub use resolver::*;
