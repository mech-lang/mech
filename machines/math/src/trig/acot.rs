use crate::*;

// Acot ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::atan;
#[cfg(feature = "f32")]
use libm::atanf;
#[cfg(feature = "f64")]
macro_rules! acot_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atan(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! acot_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atan(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acotf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atanf(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acotf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atanf(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcot, f32, acotf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcot, f64, acot);

impl_canonical_math_float_unop_specializer!(MathAcot, MathAcot, "math/acot");
