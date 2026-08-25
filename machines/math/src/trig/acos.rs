use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Acos ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::acos;
#[cfg(feature = "f32")]
use libm::acosf;
#[cfg(feature = "f64")]
macro_rules! acos_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acos((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! acos_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acos(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acosf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acosf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acosf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acosf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcos, f32, acosf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcos, f64, acos);

#[cfg(feature = "source")]
fn impl_acos_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathAcos,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathAcos {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAcos {
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
        match impl_acos_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_acos_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/acos".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
