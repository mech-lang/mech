use crate::*;

// Log10 ------------------------------------------------------------------------

use libm::{log10, log10f};
macro_rules! log10_op {
    (@managed $arg:expr) => {
        Ok(log10(($arg)))
    };
}

macro_rules! log10f_op {
    (@managed $arg:expr) => {
        Ok(log10f(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog10, f32, log10f);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog10, f64, log10);

impl_canonical_math_float_unop_specializer!(MathLog10, MathLog10, "math/log10");
