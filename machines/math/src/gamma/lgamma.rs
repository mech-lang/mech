use crate::*;

// Lgamma ------------------------------------------------------------------------

use libm::{lgamma, lgammaf};
macro_rules! lgamma_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = lgamma((*$arg));
        }
    };
}

macro_rules! lgamma_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = lgamma(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! lgammaf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = lgammaf((*$arg));
        }
    };
}

macro_rules! lgammaf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = lgammaf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLgamma, f32, lgammaf);
#[cfg(feature = "f64")]
impl_math_unop!(MathLgamma, f64, lgamma);

impl_canonical_math_float_unop_specializer!(MathLgamma, MathLgamma, "math/lgamma");
