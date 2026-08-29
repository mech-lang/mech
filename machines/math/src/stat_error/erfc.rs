use crate::*;

// Erfc ------------------------------------------------------------------------

use libm::{erfc, erfcf};
macro_rules! erfc_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = erfc((*$arg));
        }
    };
}

macro_rules! erfc_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = erfc(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! erfcf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = erfcf((*$arg));
        }
    };
}

macro_rules! erfcf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = erfcf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathErfc, f32, erfcf);
#[cfg(feature = "f64")]
impl_math_unop!(MathErfc, f64, erfc);

impl_canonical_math_float_unop_specializer!(MathErfc, MathErfc, "math/erfc");
