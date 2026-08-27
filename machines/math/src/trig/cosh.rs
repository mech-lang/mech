use crate::*;
#[cfg(feature = "f64")]
use libm::cosh;
#[cfg(feature = "f32")]
use libm::coshf;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
#[cfg(feature = "source")]
use num_traits::*;

// Cosh ------------------------------------------------------------------------
#[cfg(feature = "f64")]
macro_rules! cosh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = cosh((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! cosh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = cosh(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! coshf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = coshf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! coshf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = coshf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCosh, f32, coshf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCosh, f64, cosh);

#[cfg(feature = "source")]
fn impl_cosh_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathCosh,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathCosh {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathCosh {
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
        match impl_cosh_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_cosh_fxn(input.borrow().clone()),
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: input.kind(),
                        fxn_name: "math/cosh".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
