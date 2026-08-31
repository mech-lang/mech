use crate::*;

// J0 ------------------------------------------------------------------------

use libm::{j0, j0f};
macro_rules! j0_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = j0((*$arg));
        }
    };
}

macro_rules! j0_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = j0(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! j0f_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = j0f((*$arg));
        }
    };
}

macro_rules! j0f_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = j0f(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathJ0, f32, j0f);
#[cfg(feature = "f64")]
impl_math_unop!(MathJ0, f64, j0);

impl_canonical_math_float_unop_specializer!(MathJ0, MathJ0, "math/bessel/j0");
