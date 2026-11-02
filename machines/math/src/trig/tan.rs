use crate::*;

// Tan ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::tan;
#[cfg(feature = "f32")]
use libm::tanf;
#[cfg(feature = "f64")]
macro_rules! tan_op {
    (@managed $arg:expr) => {
        Ok(tan(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! tanf_op {
    (@managed $arg:expr) => {
        Ok(tanf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTan, f32, tanf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTan, f64, tan);

impl_canonical_math_float_unop_specializer!(MathTan, MathTan, "math/tan");
