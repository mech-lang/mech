use crate::*;
#[cfg(feature = "f64")]
use libm::tanh;
#[cfg(feature = "f32")]
use libm::tanhf;

// Tanh ------------------------------------------------------------------------
#[cfg(feature = "f64")]
macro_rules! tanh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = tanh((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! tanh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = tanh(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! tanhf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = tanhf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! tanhf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = tanhf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTanh, f32, tanhf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTanh, f64, tanh);

impl_canonical_math_float_unop_specializer!(MathTanh, MathTanh, "math/tanh");
