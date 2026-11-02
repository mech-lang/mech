use crate::*;
#[cfg(feature = "f64")]
use libm::atanh;
#[cfg(feature = "f32")]
use libm::atanhf;

// Atanh Macros
#[cfg(feature = "f64")]
macro_rules! atanh_op {
    (@managed $arg:expr) => {
        Ok(atanh(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! atanhf_op {
    (@managed $arg:expr) => {
        Ok(atanhf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAtanh, f32, atanhf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAtanh, f64, atanh);

impl_canonical_math_float_unop_specializer!(MathAtanh, MathAtanh, "math/atanh");
