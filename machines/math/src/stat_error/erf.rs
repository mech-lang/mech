use crate::*;

// Erf ------------------------------------------------------------------------

use libm::{erf, erff};
macro_rules! erf_op {
    (@managed $arg:expr) => {
        Ok(erf(($arg)))
    };
}

macro_rules! erff_op {
    (@managed $arg:expr) => {
        Ok(erff(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathErf, f32, erff);
#[cfg(feature = "f64")]
impl_math_unop!(MathErf, f64, erf);

impl_canonical_math_float_unop_specializer!(MathErf, MathErf, "math/erf");
