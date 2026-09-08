use crate::*;

// Y0 ------------------------------------------------------------------------

use libm::{y0, y0f};
macro_rules! y0_op {
    (@managed $arg:expr) => {
        Ok(y0(($arg)))
    };
}

macro_rules! y0f_op {
    (@managed $arg:expr) => {
        Ok(y0f(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathY0, f32, y0f);
#[cfg(feature = "f64")]
impl_math_unop!(MathY0, f64, y0);

impl_canonical_math_float_unop_specializer!(MathY0, MathY0, "math/bessel/y0");
