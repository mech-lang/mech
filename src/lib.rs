extern crate nalgebra as na;
extern crate mech_core;

use na::*;
use num_traits::*;
use std::ops::*;
use std::fmt::Debug;
use mech_core::matrix::Matrix;

pub mod n_choose_k;

pub use self::n_choose_k::*;
