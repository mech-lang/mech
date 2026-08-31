use crate::*;

// J1 ------------------------------------------------------------------------

use libm::{j1, j1f};
macro_rules! j1_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = j1((*$arg));
        }
    };
}

macro_rules! j1_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = j1(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! j1f_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = j1f((*$arg));
        }
    };
}

macro_rules! j1f_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = j1f(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathJ1, f32, j1f);
#[cfg(feature = "f64")]
impl_math_unop!(MathJ1, f64, j1);

impl_canonical_math_float_unop_specializer!(MathJ1, MathJ1, "math/bessel/j1");
