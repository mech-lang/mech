use crate::*;

// Lgamma ------------------------------------------------------------------------

use libm::{lgamma, lgammaf};
macro_rules! lgamma_op {
    (@managed $arg:expr) => {
        Ok(lgamma(($arg)))
    };
}

macro_rules! lgammaf_op {
    (@managed $arg:expr) => {
        Ok(lgammaf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLgamma, f32, lgammaf);
#[cfg(feature = "f64")]
impl_math_unop!(MathLgamma, f64, lgamma);

impl_canonical_math_float_unop_specializer!(MathLgamma, MathLgamma, "math/lgamma");
