use crate::*;

// Erf ------------------------------------------------------------------------

use libm::{erf, erff};
macro_rules! erf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = erf((*$arg));
        }
    };
}

macro_rules! erf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = erf(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! erff_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = erff((*$arg));
        }
    };
}

macro_rules! erff_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = erff(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathErf, f32, erff);
#[cfg(feature = "f64")]
impl_math_unop!(MathErf, f64, erf);

impl_canonical_math_float_unop_specializer!(MathErf, MathErf, "math/erf");
