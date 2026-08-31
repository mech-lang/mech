use crate::*;

// Log ------------------------------------------------------------------------

use libm::{log, logf};
macro_rules! log_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = log((*$arg));
        }
    };
}

macro_rules! log_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = log(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! logf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = logf((*$arg));
        }
    };
}

macro_rules! logf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = logf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog, f32, logf);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog, f64, log);

impl_canonical_math_float_unop_specializer!(MathLog, MathLog, "math/log");
