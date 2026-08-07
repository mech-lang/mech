use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Exp2 ------------------------------------------------------------------------

use libm::{exp2, exp2f};
macro_rules! exp2_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = exp2((*$arg));
        }
    };
}

macro_rules! exp2_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = exp2(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! exp2f_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = exp2f((*$arg));
        }
    };
}

macro_rules! exp2f_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = exp2f(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f64")]
impl_math_unop!(
    MathExp2,
    f64,
    exp2,
    FeatureFlag::Custom(hash_str("math/exp2"))
);
#[cfg(feature = "f32")]
impl_math_unop!(
    MathExp2,
    f32,
    exp2f,
    FeatureFlag::Custom(hash_str("math/exp2"))
);

#[cfg(feature = "source")]
fn impl_exp2_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathExp2,
      lhs_value,
      F32 => MatrixF32, F32, f32::default(), "f32";
      F64 => MatrixF64, F64, f64::default(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathExp2 {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathExp2 {
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
        match impl_exp2_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                Value::MutableReference(input) => impl_exp2_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/exp2".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
