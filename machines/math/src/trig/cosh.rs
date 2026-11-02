use crate::*;
#[cfg(feature = "f64")]
use libm::cosh;
#[cfg(feature = "f32")]
use libm::coshf;

// Cosh ------------------------------------------------------------------------
#[cfg(feature = "f64")]
macro_rules! cosh_op {
    (@managed $arg:expr) => {
        Ok(cosh(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! coshf_op {
    (@managed $arg:expr) => {
        Ok(coshf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCosh, f32, coshf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCosh, f64, cosh);

impl_canonical_math_float_unop_specializer!(MathCosh, MathCosh, "math/cosh");
