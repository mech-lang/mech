use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Atan ------------------------------------------------------------------------

use libm::{atan, atanf};
macro_rules! atan_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atan((*$arg));
        }
    };
}

macro_rules! atan_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atan(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! atanf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = atanf((*$arg));
        }
    };
}

macro_rules! atanf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = atanf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAtan, f32, atanf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAtan, f64, atan);

#[cfg(feature = "source")]
fn impl_atan_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathAtan,
      (lhs_value),
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathAtan {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAtan {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() == 1 {
            let input = arguments[0].clone();
            match impl_atan_fxn(input.clone()) {
                Ok(fxn) => Ok(fxn),
                Err(_) => match (input) {
                    (LegacyValue::MutableReference(input)) => impl_atan_fxn(input.borrow().clone()),
                    x => Err(MechError::new(
                        UnhandledFunctionArgumentKind1 {
                            arg: x.kind(),
                            fxn_name: "math/atan".to_string(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                },
            }
        } else if arguments.len() == 2 {
            let arg1 = arguments[0].clone();
            let arg2 = arguments[1].clone();
            match impl_atan2_fxn(arg1.clone(), arg2.clone()) {
                Ok(fxn) => Ok(fxn),
                Err(_) => match (arg1, arg2) {
                    (LegacyValue::MutableReference(arg1), LegacyValue::MutableReference(arg2)) => {
                        impl_atan2_fxn(arg1.borrow().clone(), arg2.borrow().clone())
                    }
                    (LegacyValue::MutableReference(arg1), arg2) => {
                        impl_atan2_fxn(arg1.borrow().clone(), arg2.clone())
                    }
                    (arg1, LegacyValue::MutableReference(arg2)) => {
                        impl_atan2_fxn(arg1.clone(), arg2.borrow().clone())
                    }
                    (arg1, arg2) => Err(MechError::new(
                        UnhandledFunctionArgumentKind2 {
                            arg: (arg1.kind(), arg2.kind()),
                            fxn_name: "math/atan".to_string(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                },
            }
        } else {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
    }
}
