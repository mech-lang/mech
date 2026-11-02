use crate::*;

// Tgamma ------------------------------------------------------------------------

use libm::{tgamma, tgammaf};
macro_rules! tgamma_op {
    (@managed $arg:expr) => {
        Ok(tgamma(($arg)))
    };
}

macro_rules! tgammaf_op {
    (@managed $arg:expr) => {
        Ok(tgammaf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTgamma, f32, tgammaf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTgamma, f64, tgamma);

impl_canonical_math_float_unop_specializer!(MathTgamma, MathTgamma, "math/tgamma");
