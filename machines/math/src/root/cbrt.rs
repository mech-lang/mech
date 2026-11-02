use crate::*;

// Cbrt ------------------------------------------------------------------------

use libm::{cbrt, cbrtf};
macro_rules! cbrt_op {
    (@managed $arg:expr) => {
        Ok(cbrt(($arg)))
    };
}

macro_rules! cbrtf_op {
    (@managed $arg:expr) => {
        Ok(cbrtf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCbrt, f32, cbrtf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCbrt, f64, cbrt);

impl_canonical_math_float_unop_specializer!(MathCbrt, MathCbrt, "math/cbrt");
