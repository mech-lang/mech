use crate::*;

// Y1 ------------------------------------------------------------------------

use libm::{y1, y1f};
macro_rules! y1_op {
    (@managed $arg:expr) => {
        Ok(y1(($arg)))
    };
}

macro_rules! y1f_op {
    (@managed $arg:expr) => {
        Ok(y1f(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathY1, f32, y1f);
#[cfg(feature = "f64")]
impl_math_unop!(MathY1, f64, y1);

impl_canonical_math_float_unop_specializer!(MathY1, MathY1, "math/bessel/y1");
