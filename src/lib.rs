#![no_main]
#![allow(warnings)]

#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate mech_core;

#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

#[cfg(feature = "n_choose_k")]
pub mod n_choose_k;

#[cfg(feature = "n_choose_k")]
pub use self::n_choose_k::*;
