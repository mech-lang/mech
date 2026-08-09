use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Log2 ------------------------------------------------------------------------

use libm::{log2, log2f};
macro_rules! log2_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = log2((*$arg));
        }
    };
}

macro_rules! log2_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = log2(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! log2f_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = log2f((*$arg));
        }
    };
}

macro_rules! log2f_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = log2f(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog2, f32, log2f);
#[cfg(feature = "f64")]
impl_math_unop!(MathLog2, f64, log2);

#[cfg(feature = "source")]
fn impl_log2_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathLog2,
      (lhs_value),
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathLog2 {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathLog2 {
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
        match impl_log2_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (input) {
                (LegacyValue::MutableReference(input)) => impl_log2_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/log2".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
