use crate::*;

// Ilogb ------------------------------------------------------------------------

use libm::{ilogb, ilogbf};
macro_rules! ilogb_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = ilogb((*$arg));
        }
    };
}

macro_rules! ilogb_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = ilogb(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! ilogbf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = ilogbf((*$arg));
        }
    };
}

macro_rules! ilogbf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = ilogbf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathIlogb, f32, ilogbf);
#[cfg(feature = "f64")]
impl_math_unop!(MathIlogb, f64, ilogb);
