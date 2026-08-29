use crate::*;

// Log2 ------------------------------------------------------------------------

use libm::{log2, log2f};
macro_rules! log2_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = log2((*$arg));
        }
    };
}

macro_rules! log2_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = log2(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! log2f_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = log2f((*$arg));
        }
    };
}

macro_rules! log2f_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = log2f(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog2, f32, log2f);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog2, f64, log2);

impl_canonical_math_float_unop_specializer!(MathLog2, MathLog2, "math/log2");
