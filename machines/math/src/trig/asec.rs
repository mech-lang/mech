use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
#[cfg(feature = "source")]
use num_traits::*;

// Asec ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::acos;
#[cfg(feature = "f32")]
use libm::acosf;
#[cfg(feature = "f64")]
macro_rules! asec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acos(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! asec_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acos(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! asecf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = acosf(1.0 / (*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! asecf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = acosf(1.0 / ((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAsec, f32, asecf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAsec, f64, asec);

#[cfg(feature = "source")]
fn impl_asec_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathAsec,
      lhs_value,
      F32 => MatrixF32, F32, f32::zero(), "f32";
      F64 => MatrixF64, F64, f64::zero(), "f64";
    )
}

#[cfg(feature = "source")]
pub struct MathAsec {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAsec {
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
        match impl_asec_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(input) => impl_asec_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/asec".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
