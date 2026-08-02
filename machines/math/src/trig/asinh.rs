use crate::*;
use libm::{asinh, asinhf};
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Asinh Macros
macro_rules! asinh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asinh((*$arg));
        }
    };
}

macro_rules! asinh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asinh(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! asinhf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asinhf((*$arg));
        }
    };
}

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

#[cfg(feature = "source")]
fn impl_asinh_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathAsinh,
      (lhs_value),
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathAsinh {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAsinh {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = arguments[0].clone();
        match impl_asinh_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                Value::MutableReference(input) => impl_asinh_fxn(input.borrow().clone()),
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: input.kind(),
                        fxn_name: "math/asinh".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
