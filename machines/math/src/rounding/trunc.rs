use crate::*;

// Trunc ------------------------------------------------------------------------

use libm::{trunc, truncf};
macro_rules! trunc_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = trunc((*$arg));
        }
    };
}

macro_rules! trunc_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = trunc(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! truncf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = truncf((*$arg));
        }
    };
}

macro_rules! truncf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = truncf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTrunc, f32, truncf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTrunc, f64, trunc);

impl_canonical_math_float_unop_specializer!(MathTrunc, MathTrunc, "math/trunc");
