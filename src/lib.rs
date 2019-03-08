// # Syntax

#![feature(alloc)]
#![feature(drain_filter)]

extern crate mech_core;
#[macro_use]
extern crate alloc;
extern crate hashbrown;
#[macro_use]
extern crate nom;

pub mod lexer;
#[macro_use]
pub mod parser;
pub mod compiler;
pub mod formatter;