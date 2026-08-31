use crate::*;

// Y0 ------------------------------------------------------------------------

use libm::{y0, y0f};
macro_rules! y0_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = y0((*$arg));
        }
    };
}

macro_rules! y0_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = y0(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! y0f_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = y0f((*$arg));
        }
    };
}

macro_rules! y0f_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = y0f(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathY0, f32, y0f);
#[cfg(feature = "f64")]
impl_math_unop!(MathY0, f64, y0);

impl_canonical_math_float_unop_specializer!(MathY0, MathY0, "math/bessel/y0");
