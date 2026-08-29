use crate::*;

// Csc ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::sin;
#[cfg(feature = "f32")]
use libm::sinf;
#[cfg(feature = "f64")]
macro_rules! csc_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / sin((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! csc_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / sin(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! cscf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / sinf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! cscf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / sinf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCsc, f32, cscf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCsc, f64, csc);

impl_canonical_math_float_unop_specializer!(MathCsc, MathCsc, "math/csc");
