use crate::*;

// Sec ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::cos;
#[cfg(feature = "f32")]
use libm::cosf;
#[cfg(feature = "f64")]
macro_rules! sec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / cos((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! sec_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / cos(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! secf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / cosf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! secf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / cosf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSec, f32, secf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSec, f64, sec);

impl_canonical_math_float_unop_specializer!(MathSec, MathSec, "math/sec");
