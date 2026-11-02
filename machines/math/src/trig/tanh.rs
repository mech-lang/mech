use crate::*;
#[cfg(feature = "f64")]
use libm::tanh;
#[cfg(feature = "f32")]
use libm::tanhf;

// Tanh ------------------------------------------------------------------------
#[cfg(feature = "f64")]
macro_rules! tanh_op {
    (@managed $arg:expr) => {
        Ok(tanh(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! tanhf_op {
    (@managed $arg:expr) => {
        Ok(tanhf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTanh, f32, tanhf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTanh, f64, tanh);

impl_canonical_math_float_unop_specializer!(MathTanh, MathTanh, "math/tanh");
