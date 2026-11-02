use crate::*;

// Sec ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::cos;
#[cfg(feature = "f32")]
use libm::cosf;
#[cfg(feature = "f64")]
macro_rules! sec_op {
    (@managed $arg:expr) => {
        Ok(1.0 / cos(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! secf_op {
    (@managed $arg:expr) => {
        Ok(1.0 / cosf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSec, f32, secf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSec, f64, sec);

impl_canonical_math_float_unop_specializer!(MathSec, MathSec, "math/sec");
