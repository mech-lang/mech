use crate::*;

// Sqrt ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::sqrt;
#[cfg(feature = "f32")]
use libm::sqrtf;
#[cfg(feature = "f64")]
macro_rules! sqrt_op {
    (@managed $arg:expr) => {
        Ok(sqrt(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! sqrtf_op {
    (@managed $arg:expr) => {
        Ok(sqrtf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSqrt, f32, sqrtf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSqrt, f64, sqrt);

impl_canonical_math_float_unop_specializer!(MathSqrt, MathSqrt, "math/sqrt");
