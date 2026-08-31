use crate::*;

// Sin ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::sin;
#[cfg(feature = "f32")]
use libm::sinf;
#[cfg(feature = "f64")]
macro_rules! sin_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sin((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! sin_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sin(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! sinf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sinf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! sinf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sinf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSin, f32, sinf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSin, f64, sin);

impl_canonical_math_float_unop_specializer!(MathSin, MathSin, "math/sin");
