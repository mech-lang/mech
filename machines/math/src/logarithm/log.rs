use crate::*;

// Log ------------------------------------------------------------------------

use libm::{log, logf};
macro_rules! log_op {
    (@managed $arg:expr) => {
        Ok(log(($arg)))
    };
}

macro_rules! logf_op {
    (@managed $arg:expr) => {
        Ok(logf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog, f32, logf);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog, f64, log);

impl_canonical_math_float_unop_specializer!(MathLog, MathLog, "math/log");
