use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
#[cfg(feature = "source")]
use num_traits::*;

// Cot ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::tan;
#[cfg(feature = "f32")]
use libm::tanf;
#[cfg(feature = "f64")]
macro_rules! cot_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / tan((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! cot_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / tan(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! cotf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = 1.0 / tanf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! cotf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = 1.0 / tanf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCot, f32, cotf);
#[cfg(feature = "f64")]
impl_math_unop!(MathCot, f64, cot);

#[cfg(feature = "source")]
fn impl_cot_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathCot,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathCot {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathCot {
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
        match impl_cot_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_cot_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/cot".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
