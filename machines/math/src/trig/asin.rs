use crate::*;

// Asin ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::asin;
#[cfg(feature = "f32")]
use libm::asinf;
#[cfg(feature = "f64")]
macro_rules! asin_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asin((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! asin_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asin(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! asinf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asinf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! asinf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asinf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAsin, f32, asinf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAsin, f64, asin);

impl_canonical_math_float_unop_specializer!(MathAsin, MathAsin, "math/asin");
