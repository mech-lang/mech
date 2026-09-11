use crate::*;

// Acot ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::atan;
#[cfg(feature = "f32")]
use libm::atanf;
#[cfg(feature = "f64")]
macro_rules! acot_op {
    (@managed $arg:expr) => {
        Ok(atan(1.0 / ($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! acotf_op {
    (@managed $arg:expr) => {
        Ok(atanf(1.0 / ($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcot, f32, acotf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcot, f64, acot);

impl_canonical_math_float_unop_specializer!(MathAcot, MathAcot, "math/acot");
