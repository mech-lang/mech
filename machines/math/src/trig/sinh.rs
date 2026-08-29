use crate::*;
#[cfg(feature = "f64")]
use libm::sinh;
#[cfg(feature = "f32")]
use libm::sinhf;

// Sinh ------------------------------------------------------------------------
#[cfg(feature = "f64")]
macro_rules! sinh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sinh((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! sinh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sinh(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! sinhf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sinhf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! sinhf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sinhf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSinh, f32, sinhf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSinh, f64, sinh);

impl_canonical_math_float_unop_specializer!(MathSinh, MathSinh, "math/sinh");
