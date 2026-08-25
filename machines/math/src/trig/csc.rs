use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Csc ------------------------------------------------------------------------

use libm::{sin, sinf};
macro_rules! csc_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / sin((*$arg));
        }
    };
}

macro_rules! csc_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / sin(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! cscf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / sinf((*$arg));
        }
    };
}

macro_rules! cscf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / sinf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCsc, f32, cscf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCsc, f64, csc);

#[cfg(feature = "source")]
fn impl_csc_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathCsc,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathCsc {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathCsc {
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
        match impl_csc_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_csc_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/csc".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
