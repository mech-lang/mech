use crate::*;

// Trunc ------------------------------------------------------------------------

use libm::{trunc, truncf};
macro_rules! trunc_op {
    (@managed $arg:expr) => {
        Ok(trunc(($arg)))
    };
}

macro_rules! truncf_op {
    (@managed $arg:expr) => {
        Ok(truncf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTrunc, f32, truncf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTrunc, f64, trunc);

impl_canonical_math_float_unop_specializer!(MathTrunc, MathTrunc, "math/trunc");
