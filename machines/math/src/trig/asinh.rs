use crate::*;
#[cfg(feature = "f64")]
use libm::asinh;
#[cfg(feature = "f32")]
use libm::asinhf;

// Asinh Macros
#[cfg(feature = "f64")]
macro_rules! asinh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asinh((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! asinh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asinh(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! asinhf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asinhf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! asinhf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asinhf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAsinh, f32, asinhf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAsinh, f64, asinh);

impl_canonical_math_float_unop_specializer!(MathAsinh, MathAsinh, "math/asinh");
