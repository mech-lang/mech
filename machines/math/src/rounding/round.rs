use crate::*;

// Round ------------------------------------------------------------------------

use libm::{round, roundf};
macro_rules! round_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = round((*$arg));
        }
    };
}

macro_rules! round_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = round(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! roundf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = roundf((*$arg));
        }
    };
}

macro_rules! roundf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = roundf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathRound, f32, roundf);
#[cfg(feature = "f64")]
impl_math_unop!(MathRound, f64, round);

impl_canonical_math_float_unop_specializer!(MathRound, MathRound, "math/round");
