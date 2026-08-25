use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
#[cfg(feature = "source")]
use num_traits::*;

// Sqrt ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::sqrt;
#[cfg(feature = "f32")]
use libm::sqrtf;
#[cfg(feature = "f64")]
macro_rules! sqrt_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sqrt((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! sqrt_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sqrt(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! sqrtf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = sqrtf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! sqrtf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = sqrtf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSqrt, f32, sqrtf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSqrt, f64, sqrt);

#[cfg(feature = "source")]
fn impl_sqrt_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathSqrt,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathSqrt {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathSqrt {
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
        match impl_sqrt_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_sqrt_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/sqrt".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
