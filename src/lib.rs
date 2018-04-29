// # Mech

/*
Mech is a programming language especially suited for developing reactive 
systems. 
*/

// ## Prelude

#![cfg_attr(target_os = "none", no_std)]
#![feature(alloc)]

extern crate rlibc;
#[macro_use]
extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate core;
extern crate hashmap_core;
extern crate rand;
extern crate time;

// ## Modules

pub mod database;
pub mod runtime;
pub mod table;
pub mod indexes;
pub mod operations;