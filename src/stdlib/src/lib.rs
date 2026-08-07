#![cfg_attr(feature = "no_std", no_std)]

#[cfg(feature = "no_std")]
extern crate alloc;

mod catalog;

pub use catalog::*;
