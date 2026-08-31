use crate::*;

// Sincos ------------------------------------------------------------------------

use libm::{sincos, sincosf};
macro_rules! sincos_op {
    ($arg:expr, $out1:expr, $out2:expr) => {
        unsafe {
            (*$out1, *$out2) = sincos((*$arg));
        }
    };
}

macro_rules! sincos_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sincos(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! sincosf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sincosf((*$arg));
        }
    };
}

macro_rules! sincosf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sincosf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSincos, f32, sincosf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSincos, f64, sincos);
