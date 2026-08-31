use crate::*;

// Rint ------------------------------------------------------------------------

use libm::{rint, rintf};
macro_rules! rint_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = rint((*$arg));
        }
    };
}

macro_rules! rint_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = rint(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! rintf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = rintf((*$arg));
        }
    };
}

macro_rules! rintf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = rintf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathRint, f32, rintf);
#[cfg(feature = "f64")]
impl_math_unop!(MathRint, f64, rint);

impl_canonical_math_float_unop_specializer!(MathRint, MathRint, "math/rint");
