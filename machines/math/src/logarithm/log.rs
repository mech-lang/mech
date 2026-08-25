use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Log ------------------------------------------------------------------------

use libm::{log, logf};
macro_rules! log_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = log((*$arg));
        }
    };
}

macro_rules! log_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = log(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! logf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = logf((*$arg));
        }
    };
}

macro_rules! logf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = logf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog, f32, logf);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog, f64, log);

#[cfg(feature = "source")]
fn impl_log_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathLog,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathLog {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathLog {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        match impl_log_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_log_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/log".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
