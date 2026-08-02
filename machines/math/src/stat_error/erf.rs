use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Erf ------------------------------------------------------------------------

use libm::{erf, erff};
macro_rules! erf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = erf((*$arg));
        }
    };
}

macro_rules! erf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = erf(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! erff_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = erff((*$arg));
        }
    };
}

macro_rules! erff_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = erff(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathErf, f32, erff);
#[cfg(feature = "f64")]
impl_math_unop!(MathErf, f64, erf);

#[cfg(feature = "source")]
fn impl_erf_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathErf,
      (lhs_value),
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathErf {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathErf {
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
        match impl_erf_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (input) {
                (Value::MutableReference(input)) => impl_erf_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/erf".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
