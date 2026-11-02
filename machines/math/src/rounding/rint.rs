use crate::*;

// Rint ------------------------------------------------------------------------

use libm::{rint, rintf};
macro_rules! rint_op {
    (@managed $arg:expr) => {
        Ok(rint(($arg)))
    };
}

macro_rules! rintf_op {
    (@managed $arg:expr) => {
        Ok(rintf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathRint, f32, rintf);
#[cfg(feature = "f64")]
impl_math_unop!(MathRint, f64, rint);

impl_canonical_math_float_unop_specializer!(MathRint, MathRint, "math/rint");
