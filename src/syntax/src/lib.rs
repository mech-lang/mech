// # Syntax

#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![feature(extract_if)]
#![feature(get_mut_unchecked)]
#![allow(dead_code)]
#![allow(warnings)]

extern crate mech_core;
#[cfg(feature="no-std")] #[macro_use] extern crate alloc;
#[cfg(not(feature = "no-std"))] extern crate core;
extern crate hashbrown;
extern crate nom;
extern crate nom_unicode;
#[macro_use]
extern crate lazy_static;

pub mod parser;
pub mod parser2;
pub mod ast;
pub mod compiler;
pub mod analyzer;
#[cfg(feature="lang-server")]
pub mod langserver;
pub mod formatter;
