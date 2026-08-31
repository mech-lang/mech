use crate::*;

// Acos ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::acos;
#[cfg(feature = "f32")]
use libm::acosf;
#[cfg(feature = "f64")]
macro_rules! acos_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acos((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! acos_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acos(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acosf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acosf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acosf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acosf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcos, f32, acosf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcos, f64, acos);

impl_canonical_math_float_unop_specializer!(MathAcos, MathAcos, "math/acos");
