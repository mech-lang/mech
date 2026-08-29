use crate::*;

// Cot ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::tan;
#[cfg(feature = "f32")]
use libm::tanf;
#[cfg(feature = "f64")]
macro_rules! cot_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / tan((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! cot_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / tan(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! cotf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / tanf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! cotf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / tanf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCot, f32, cotf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCot, f64, cot);

impl_canonical_math_float_unop_specializer!(MathCot, MathCot, "math/cot");
