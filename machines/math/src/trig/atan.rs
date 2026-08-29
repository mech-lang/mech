use crate::*;

// Atan ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::atan;
#[cfg(feature = "f32")]
use libm::atanf;
#[cfg(feature = "f64")]
macro_rules! atan_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atan((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! atan_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atan(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! atanf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atanf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! atanf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atanf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAtan, f32, atanf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAtan, f64, atan);

impl_canonical_math_float_unop_specializer!(MathAtan, MathAtan, "math/atan");
