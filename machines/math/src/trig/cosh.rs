use crate::*;
#[cfg(feature = "f64")]
use libm::cosh;
#[cfg(feature = "f32")]
use libm::coshf;

// Cosh ------------------------------------------------------------------------
#[cfg(feature = "f64")]
macro_rules! cosh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = cosh((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! cosh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = cosh(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! coshf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = coshf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! coshf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = coshf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCosh, f32, coshf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCosh, f64, cosh);

impl_canonical_math_float_unop_specializer!(MathCosh, MathCosh, "math/cosh");
