use crate::*;
#[cfg(feature = "f64")]
use libm::tanh;
#[cfg(feature = "f32")]
use libm::tanhf;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Tanh ------------------------------------------------------------------------
#[cfg(feature = "f64")]
macro_rules! tanh_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = tanh((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! tanh_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = tanh(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! tanhf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = tanhf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! tanhf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = tanhf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTanh, f32, tanhf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTanh, f64, tanh);

#[cfg(feature = "source")]
fn impl_tanh_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathTanh,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathTanh {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathTanh {
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
        match impl_tanh_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_tanh_fxn(input.borrow().clone()),
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: input.kind(),
                        fxn_name: "math/tanh".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
