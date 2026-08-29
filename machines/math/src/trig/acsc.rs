use crate::*;

// Acsc ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::asin;
#[cfg(feature = "f32")]
use libm::asinf;
#[cfg(feature = "f64")]
macro_rules! acsc_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asin(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! acsc_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asin(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acscf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asinf(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acscf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asinf(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcsc, f32, acscf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcsc, f64, acsc);

impl_canonical_math_float_unop_specializer!(MathAcsc, MathAcsc, "math/acsc");
