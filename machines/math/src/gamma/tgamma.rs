use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Tgamma ------------------------------------------------------------------------

use libm::{tgamma, tgammaf};
macro_rules! tgamma_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = tgamma((*$arg));
        }
    };
}

macro_rules! tgamma_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = tgamma(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! tgammaf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = tgammaf((*$arg));
        }
    };
}

macro_rules! tgammaf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = tgammaf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTgamma, f32, tgammaf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTgamma, f64, tgamma);

#[cfg(feature = "source")]
fn impl_tgamma_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathTgamma,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathTgamma {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathTgamma {
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
        match impl_tgamma_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_tgamma_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/tgamma".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
