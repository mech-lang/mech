use crate::*;

// Ceil ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::ceil;
#[cfg(feature = "f32")]
use libm::ceilf;
#[cfg(feature = "f64")]
macro_rules! ceil_op {
    (@managed $arg:expr) => {
        Ok(ceil(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! ceilf_op {
    (@managed $arg:expr) => {
        Ok(ceilf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCeil, f32, ceilf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCeil, f64, ceil);

impl_canonical_math_float_unop_specializer!(MathCeil, MathCeil, "math/ceil");
