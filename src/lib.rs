// # Syntax

#![feature(alloc)]

extern crate mech;
#[macro_use]
extern crate alloc;

pub mod lexer;
#[macro_use]
pub mod parser;
pub mod compiler;
pub mod formatter;