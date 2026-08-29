use crate::*;
#[cfg(feature = "f64")]
use libm::atanh;
#[cfg(feature = "f32")]
use libm::atanhf;

// Atanh Macros
#[cfg(feature = "f64")]
macro_rules! atanh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atanh((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! atanh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atanh(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! atanhf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atanhf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! atanhf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atanhf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAtanh, f32, atanhf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAtanh, f64, atanh);

impl_canonical_math_float_unop_specializer!(MathAtanh, MathAtanh, "math/atanh");
