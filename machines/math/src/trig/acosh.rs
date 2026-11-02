use crate::*;
#[cfg(feature = "f64")]
use libm::acosh;
#[cfg(feature = "f32")]
use libm::acoshf;

// Acosh Macros
#[cfg(feature = "f64")]
macro_rules! acosh_op {
    (@managed $arg:expr) => {
        Ok(acosh(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! acoshf_op {
    (@managed $arg:expr) => {
        Ok(acoshf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcosh, f32, acoshf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcosh, f64, acosh);

impl_canonical_math_float_unop_specializer!(MathAcosh, MathAcosh, "math/acosh");
