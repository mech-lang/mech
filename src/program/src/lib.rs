
// Program
// =============================================================================

#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![allow(dead_code)]
#![allow(warnings)]

#[cfg(feature = "program")]
pub mod program;
//#[cfg(feature = "runloop")]
//pub mod runloop;
//#[cfg(feature = "persister")]
//pub mod persister;

#[cfg(feature = "program")]
pub use crate::program::*;
//#[cfg(feature = "runloop")]
//pub use crate::runloop::*;
//#[cfg(feature = "persister")]
//pub use crate::persister::*;

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