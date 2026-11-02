use crate::*;

// Sin ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::sin;
#[cfg(feature = "f32")]
use libm::sinf;
#[cfg(feature = "f64")]
macro_rules! sin_op {
    (@managed $arg:expr) => {
        Ok(sin(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! sinf_op {
    (@managed $arg:expr) => {
        Ok(sinf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSin, f32, sinf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSin, f64, sin);

impl_canonical_math_float_unop_specializer!(MathSin, MathSin, "math/sin");
