use crate::*;

// Cos ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::cos;
#[cfg(feature = "f32")]
use libm::cosf;
#[cfg(feature = "f64")]
macro_rules! cos_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = cos((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! cos_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = cos(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! cosf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = cosf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! cosf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = cosf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCos, f32, cosf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCos, f64, cos);

impl_canonical_math_float_unop_specializer!(MathCos, MathCos, "math/cos");
