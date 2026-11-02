use crate::*;

// Acos ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::acos;
#[cfg(feature = "f32")]
use libm::acosf;
#[cfg(feature = "f64")]
macro_rules! acos_op {
    (@managed $arg:expr) => {
        Ok(acos(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! acosf_op {
    (@managed $arg:expr) => {
        Ok(acosf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcos, f32, acosf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcos, f64, acos);

impl_canonical_math_float_unop_specializer!(MathAcos, MathAcos, "math/acos");
