use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Roundeven ------------------------------------------------------------------------

use libm::{roundeven, roundevenf};
macro_rules! roundeven_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = roundeven((*$arg));
        }
    };
}

macro_rules! roundeven_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = roundeven(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! roundevenf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = roundevenf((*$arg));
        }
    };
}

macro_rules! roundevenf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = roundevenf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathRoundeven, f32, roundevenf);
#[cfg(feature = "f64")]
impl_math_unop!(MathRoundeven, f64, roundeven);

#[cfg(feature = "source")]
fn impl_roundeven_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathRoundeven,
      (lhs_value),
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathRoundeven {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathRoundeven {
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
        match impl_roundeven_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (input) {
                (LegacyValue::MutableReference(input)) => impl_roundeven_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/roundeven".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
