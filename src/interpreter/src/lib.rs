#![allow(warnings)]
#![feature(step_trait)]

extern crate nalgebra as na;
#[macro_use]
extern crate mech_core;

pub mod interpreter;
pub mod stdlib;

pub use crate::interpreter::*;
