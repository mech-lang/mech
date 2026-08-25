use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Acot ------------------------------------------------------------------------

use libm::{atan, atanf};
macro_rules! acot_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atan(1.0 / (*$arg));
        }
    };
}

macro_rules! acot_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atan(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! acotf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atanf(1.0 / (*$arg));
        }
    };
}

macro_rules! acotf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atanf(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcot, f32, acotf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcot, f64, acot);

#[cfg(feature = "source")]
fn impl_acot_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathAcot,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathAcot {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAcot {
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
        match impl_acot_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_acot_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/acot".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
