use crate::*;
#[cfg(feature = "f64")]
use libm::acosh;
#[cfg(feature = "f32")]
use libm::acoshf;

// Acosh Macros
#[cfg(feature = "f64")]
macro_rules! acosh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acosh((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! acosh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acosh(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acoshf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acoshf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acoshf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acoshf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcosh, f32, acoshf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcosh, f64, acosh);

impl_canonical_math_float_unop_specializer!(MathAcosh, MathAcosh, "math/acosh");
