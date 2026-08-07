#![cfg_attr(all(feature = "no_std", not(feature = "std")), no_std)]

#[cfg(feature = "no_std")]
extern crate alloc;

mod catalog;

pub use catalog::*;
