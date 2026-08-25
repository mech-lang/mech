use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
#[cfg(feature = "source")]
use num_traits::*;

// Acsc ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::asin;
#[cfg(feature = "f32")]
use libm::asinf;
#[cfg(feature = "f64")]
macro_rules! acsc_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asin(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! acsc_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asin(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acscf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = asinf(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! acscf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = asinf(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcsc, f32, acscf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAcsc, f64, acsc);

#[cfg(feature = "source")]
fn impl_acsc_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathAcsc,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathAcsc {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAcsc {
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
        match impl_acsc_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_acsc_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/acsc".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
