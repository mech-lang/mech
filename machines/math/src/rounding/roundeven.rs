use crate::*;

// Roundeven ------------------------------------------------------------------------

use libm::{roundeven, roundevenf};
macro_rules! roundeven_op {
    (@managed $arg:expr) => {
        Ok(roundeven(($arg)))
    };
}

macro_rules! roundevenf_op {
    (@managed $arg:expr) => {
        Ok(roundevenf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathRoundeven, f32, roundevenf);
#[cfg(feature = "f64")]
impl_math_unop!(MathRoundeven, f64, roundeven);

impl_canonical_math_float_unop_specializer!(MathRoundeven, MathRoundeven, "math/roundeven");
