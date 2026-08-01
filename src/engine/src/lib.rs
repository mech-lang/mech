// Program
// =============================================================================

#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(warnings)]

use mech_core::*;
pub use mech_interpreter::ExecutionServicesBorrowConflict;
#[cfg(feature = "functions")]
pub use mech_interpreter::{
    FunctionSystem, LegacyFunctionBoundary, LegacyFunctionBoundaryBuilder,
    default_function_catalog, default_function_system,
};

#[cfg(feature = "invariant_define")]
pub mod integrity;
#[cfg(feature = "native")]
pub mod native;
#[cfg(feature = "program")]
pub mod program;

#[cfg(feature = "invariant_define")]
pub use crate::integrity::*;
#[cfg(feature = "native")]
pub use crate::native::*;
#[cfg(feature = "program")]
pub use crate::program::*;

#[macro_export]
macro_rules! print_tree {
    ($tree:expr) => {
        #[cfg(feature = "pretty_print")]
        println!("{}", $tree.pretty_print());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", $tree);
    };
}

#[macro_export]
macro_rules! print_symbols {
    ($intrp:expr) => {
        #[cfg(feature = "pretty_print")]
        println!("{}", $intrp.pretty_print_symbols());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", $intrp.symbols());
    };
}

#[macro_export]
macro_rules! print_plan {
    ($intrp:expr) => {
        #[cfg(feature = "pretty_print")]
        println!("{}", $intrp.plan().pretty_print());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", $intrp.plan());
    };
}
