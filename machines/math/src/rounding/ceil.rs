use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Ceil ------------------------------------------------------------------------

use libm::{ceil, ceilf};
macro_rules! ceil_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = ceil((*$arg));
        }
    };
}

macro_rules! ceil_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = ceil(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! ceilf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = ceilf((*$arg));
        }
    };
}

macro_rules! ceilf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = ceilf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCeil, f32, ceilf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCeil, f64, ceil);

#[cfg(feature = "source")]
fn impl_ceil_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathCeil,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathCeil {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathCeil {
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
        match impl_ceil_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_ceil_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/ceil".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
