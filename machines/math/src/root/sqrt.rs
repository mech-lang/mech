use crate::*;

// Sqrt ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::sqrt;
#[cfg(feature = "f32")]
use libm::sqrtf;
#[cfg(feature = "f64")]
macro_rules! sqrt_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sqrt((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! sqrt_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sqrt(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! sqrtf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sqrtf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! sqrtf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sqrtf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSqrt, f32, sqrtf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSqrt, f64, sqrt);

impl_canonical_math_float_unop_specializer!(MathSqrt, MathSqrt, "math/sqrt");
