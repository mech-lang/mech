use crate::*;

// Floor ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::floor;
#[cfg(feature = "f32")]
use libm::floorf;
#[cfg(feature = "f64")]
macro_rules! floor_op {
    (@managed $arg:expr) => {
        Ok(floor(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! floorf_op {
    (@managed $arg:expr) => {
        Ok(floorf(($arg)))
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathFloor, f32, floorf);
#[cfg(feature = "f64")]
impl_math_unop!(MathFloor, f64, floor);

impl_canonical_math_float_unop_specializer!(MathFloor, MathFloor, "math/floor");
