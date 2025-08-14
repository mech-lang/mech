extern crate nalgebra as na;
extern crate mech_core;

use mech_core::matrix::Matrix;

#[cfg(feature = "n_choose_k")]
pub mod n_choose_k;

#[cfg(feature = "n_choose_k")]
pub use self::n_choose_k::*;
