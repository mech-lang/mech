use crate::*;

// Log1p ------------------------------------------------------------------------

use libm::{log1p, log1pf};
macro_rules! log1p_op {
    (@managed $arg:expr) => {
        Ok(log1p(($arg)))
    };
}

macro_rules! log1pf_op {
    (@managed $arg:expr) => {
        Ok(log1pf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog1p, f32, log1pf);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog1p, f64, log1p);

impl_canonical_math_float_unop_specializer!(MathLog1p, MathLog1p, "math/log1p");
