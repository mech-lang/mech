use crate::*;

// Asec ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::acos;
#[cfg(feature = "f32")]
use libm::acosf;
#[cfg(feature = "f64")]
macro_rules! asec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acos(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! asec_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acos(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! asecf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acosf(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! asecf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acosf(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAsec, f32, asecf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAsec, f64, asec);

impl_canonical_math_float_unop_specializer!(MathAsec, MathAsec, "math/asec");
