use crate::*;

// Roundeven ------------------------------------------------------------------------

use libm::{roundeven, roundevenf};
macro_rules! roundeven_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = roundeven((*$arg));
        }
    };
}

macro_rules! roundeven_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = roundeven(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! roundevenf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = roundevenf((*$arg));
        }
    };
}

macro_rules! roundevenf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = roundevenf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathRoundeven, f32, roundevenf);
#[cfg(feature = "f64")]
impl_math_unop!(MathRoundeven, f64, roundeven);

impl_canonical_math_float_unop_specializer!(MathRoundeven, MathRoundeven, "math/roundeven");
