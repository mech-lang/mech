#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
extern crate paste;

#[cfg(feature = "concat")]
pub mod concat;
