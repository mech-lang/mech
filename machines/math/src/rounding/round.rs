use crate::*;

// Round ------------------------------------------------------------------------

use libm::{round, roundf};
macro_rules! round_op {
    (@managed $arg:expr) => {
        Ok(round(($arg)))
    };
}

macro_rules! roundf_op {
    (@managed $arg:expr) => {
        Ok(roundf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathRound, f32, roundf);
#[cfg(feature = "f64")]
impl_math_unop!(MathRound, f64, round);

impl_canonical_math_float_unop_specializer!(MathRound, MathRound, "math/round");
