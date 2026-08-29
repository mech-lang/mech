use crate::*;

// Y1 ------------------------------------------------------------------------

use libm::{y1, y1f};
macro_rules! y1_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = y1((*$arg));
        }
    };
}

macro_rules! y1_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = y1(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! y1f_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = y1f((*$arg));
        }
    };
}

macro_rules! y1f_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = y1f(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathY1, f32, y1f);
#[cfg(feature = "f64")]
impl_math_unop!(MathY1, f64, y1);

impl_canonical_math_float_unop_specializer!(MathY1, MathY1, "math/bessel/y1");
