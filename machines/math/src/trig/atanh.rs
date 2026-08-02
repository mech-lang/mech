use crate::*;
use libm::{atanh, atanhf};
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Atanh Macros
macro_rules! atanh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atanh((*$arg));
        }
    };
}

macro_rules! atanh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atanh(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! atanhf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atanhf((*$arg));
        }
    };
}

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

#[cfg(feature = "source")]
fn impl_atanh_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathAtanh,
      (lhs_value),
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathAtanh {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAtanh {
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
        match impl_atanh_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                Value::MutableReference(input) => impl_atanh_fxn(input.borrow().clone()),
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: input.kind(),
                        fxn_name: "math/atanh".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
