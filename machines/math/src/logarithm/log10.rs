use crate::*;

// Log10 ------------------------------------------------------------------------

use libm::{log10, log10f};
macro_rules! log10_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = log10((*$arg));
        }
    };
}

macro_rules! log10_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = log10(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! log10f_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = log10f((*$arg));
        }
    };
}

macro_rules! log10f_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = log10f(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog10, f32, log10f);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog10, f64, log10);

impl_canonical_math_float_unop_specializer!(MathLog10, MathLog10, "math/log10");
