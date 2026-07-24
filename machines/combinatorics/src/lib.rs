#![cfg_attr(not(test), no_main)]
#![allow(warnings)]

#[cfg(feature = "matrix")]
extern crate nalgebra as na;

#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

#[cfg(any(feature = "n_choose_k", feature = "dynamic-module"))]
pub mod kernels;

#[cfg(feature = "n_choose_k")]
pub mod n_choose_k;

#[cfg(feature = "n_choose_k")]
pub use self::n_choose_k::*;

#[cfg(feature = "dynamic-module")]
mod dynamic_module;
