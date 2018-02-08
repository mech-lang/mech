// # Mech

/*
Mech is a programming language especially suited for developing reactive 
systems. 
*/

// ## Prelude

#![cfg_attr(target_os = "none", no_std)]
#![feature(alloc)]

extern crate rlibc;
extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate core;

// ## Modules

pub mod runtime;
pub mod eav;