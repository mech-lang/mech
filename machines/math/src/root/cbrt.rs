use crate::*;

// Cbrt ------------------------------------------------------------------------

use libm::{cbrt, cbrtf};
macro_rules! cbrt_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = cbrt((*$arg));
        }
    };
}

macro_rules! cbrt_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = cbrt(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! cbrtf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = cbrtf((*$arg));
        }
    };
}

macro_rules! cbrtf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = cbrtf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCbrt, f32, cbrtf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCbrt, f64, cbrt);

impl_canonical_math_float_unop_specializer!(MathCbrt, MathCbrt, "math/cbrt");
