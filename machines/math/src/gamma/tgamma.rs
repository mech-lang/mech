use crate::*;

// Tgamma ------------------------------------------------------------------------

use libm::{tgamma, tgammaf};
macro_rules! tgamma_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = tgamma((*$arg));
        }
    };
}

macro_rules! tgamma_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = tgamma(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! tgammaf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = tgammaf((*$arg));
        }
    };
}

macro_rules! tgammaf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = tgammaf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTgamma, f32, tgammaf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTgamma, f64, tgamma);

impl_canonical_math_float_unop_specializer!(MathTgamma, MathTgamma, "math/tgamma");
