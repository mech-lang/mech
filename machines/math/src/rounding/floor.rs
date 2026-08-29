use crate::*;

// Floor ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::floor;
#[cfg(feature = "f32")]
use libm::floorf;
#[cfg(feature = "f64")]
macro_rules! floor_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = floor((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! floor_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = floor(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! floorf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = floorf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! floorf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = floorf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathFloor, f32, floorf);
#[cfg(feature = "f64")]
impl_math_unop!(MathFloor, f64, floor);

impl_canonical_math_float_unop_specializer!(MathFloor, MathFloor, "math/floor");
