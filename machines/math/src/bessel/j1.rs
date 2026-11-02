use crate::*;

// J1 ------------------------------------------------------------------------

use libm::{j1, j1f};
macro_rules! j1_op {
    (@managed $arg:expr) => {
        Ok(j1(($arg)))
    };
}

macro_rules! j1f_op {
    (@managed $arg:expr) => {
        Ok(j1f(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathJ1, f32, j1f);
#[cfg(feature = "f64")]
impl_math_unop!(MathJ1, f64, j1);

impl_canonical_math_float_unop_specializer!(MathJ1, MathJ1, "math/bessel/j1");
