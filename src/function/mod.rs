use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

pub mod compare;
pub mod math_update;
pub mod math;
pub mod stats;
pub mod table;
pub mod set;
pub mod logic;