use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Sin ------------------------------------------------------------------------

use libm::{sin, sinf};
macro_rules! sin_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sin((*$arg));
        }
    };
}

macro_rules! sin_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sin(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! sinf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sinf((*$arg));
        }
    };
}

macro_rules! sinf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sinf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSin, f32, sinf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSin, f64, sin);

#[cfg(feature = "source")]
fn impl_sin_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathSin,
      (lhs_value),
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathSin {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathSin {
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
        match impl_sin_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (input) {
                (Value::MutableReference(input)) => impl_sin_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/sin".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
