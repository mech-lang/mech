use crate::*;

// Atan ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::atan;
#[cfg(feature = "f32")]
use libm::atanf;
#[cfg(feature = "f64")]
macro_rules! atan_op {
    (@managed $arg:expr) => {
        Ok(atan(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! atanf_op {
    (@managed $arg:expr) => {
        Ok(atanf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAtan, f32, atanf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAtan, f64, atan);

impl_canonical_math_float_unop_specializer!(MathAtan, MathAtan, "math/atan");
