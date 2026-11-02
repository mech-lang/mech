use crate::*;

// Log2 ------------------------------------------------------------------------

use libm::{log2, log2f};
macro_rules! log2_op {
    (@managed $arg:expr) => {
        Ok(log2(($arg)))
    };
}

macro_rules! log2f_op {
    (@managed $arg:expr) => {
        Ok(log2f(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog2, f32, log2f);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog2, f64, log2);

impl_canonical_math_float_unop_specializer!(MathLog2, MathLog2, "math/log2");
