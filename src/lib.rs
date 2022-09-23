#![no_main]
#![allow(warnings)]
extern crate mech_core;
extern crate mech_utilities;
extern crate libm;
#[macro_use]
extern crate lazy_static;

static PI: f32 = 3.14159265358979323846264338327950288;

#[macro_use]
mod macros;

pub mod sin;
pub mod cos;
pub mod tan;
pub mod atan;
pub mod atan2;

pub use self::sin::*;