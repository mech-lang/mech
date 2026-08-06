use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use num_traits::*;

// Abs ------------------------------------------------------------------------

use libm::{fabs, fabsf};

macro_rules! uabs_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$arg).clone();
        }
    };
}

macro_rules! uabs_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                (&mut (*$out))[i] = (&(*$arg))[i].clone();
            }
        }
    };
}

trait RuntimeCheckedAbs: Copy {
    fn runtime_checked_abs(self) -> Option<Self>;
}

macro_rules! impl_runtime_checked_abs {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedAbs for $type {
                fn runtime_checked_abs(self) -> Option<Self> {
                    self.checked_abs()
                }
            }
        )+
    };
}

impl_runtime_checked_abs!(i8, i16, i32, i64, i128);

fn checked_abs_value<T: RuntimeCheckedAbs>(value: T) -> MResult<T> {
    value
        .runtime_checked_abs()
        .ok_or_else(|| arithmetic_overflow::<T>("absolute value"))
}

macro_rules! checked_abs_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            let next = checked_abs_value(*$arg)?;
            *$out = next;
        }
    };
}

macro_rules! checked_abs_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            let mut next = (*$arg).clone();
            for value in next.iter_mut() {
                *value = checked_abs_value(*value)?;
            }
            *$out = next;
        }
    };
}

macro_rules! abs_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$arg).abs();
        }
    };
}

macro_rules! abs_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                (&mut (*$out))[i] = (&(*$arg))[i].abs();
            }
        }
    };
}

macro_rules! fabs_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = fabs((*$arg));
        }
    };
}

macro_rules! fabs_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = fabs(((&(*$arg))[i]));
            }
        }
    };
}

macro_rules! fabsf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = fabsf((*$arg));
        }
    };
}

macro_rules! fabsf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = fabsf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "u8")]
impl_math_unop!(MathAbs, u8, uabs);
#[cfg(feature = "u16")]
impl_math_unop!(MathAbs, u16, uabs);
#[cfg(feature = "u32")]
impl_math_unop!(MathAbs, u32, uabs);
#[cfg(feature = "u64")]
impl_math_unop!(MathAbs, u64, uabs);
#[cfg(feature = "u128")]
impl_math_unop!(MathAbs, u128, uabs);

#[cfg(feature = "i8")]
impl_math_unop!(MathAbs, i8, checked_abs);
#[cfg(feature = "i16")]
impl_math_unop!(MathAbs, i16, checked_abs);
#[cfg(feature = "i32")]
impl_math_unop!(MathAbs, i32, checked_abs);
#[cfg(feature = "i64")]
impl_math_unop!(MathAbs, i64, checked_abs);
#[cfg(feature = "i128")]
impl_math_unop!(MathAbs, i128, checked_abs);

#[cfg(feature = "f32")]
impl_math_unop!(MathAbs, f32, fabsf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAbs, f64, fabs);

#[cfg(feature = "c64")]
impl_math_unop!(MathAbs, C64, abs);

#[cfg(feature = "r64")]
impl_math_unop!(MathAbs, R64, abs);

#[cfg(all(test, feature = "i8"))]
mod checked_abs_tests {
    use super::*;

    #[test]
    fn signed_scalar_abs_rejects_minimum_and_retains_output() {
        let arg = Ref::new(7_i8);
        let out = Ref::new(19_i8);
        let function = MathAbsI8S {
            arg: arg.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 7);
        *arg.borrow_mut() = i8::MIN;

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(*out.borrow(), 7);
    }

    #[cfg(feature = "matrixd")]
    #[test]
    fn signed_matrix_abs_is_transactional_when_any_element_is_minimum() {
        let arg = Ref::new(DMatrix::from_row_slice(1, 2, &[-2_i8, 3]));
        let out = Ref::new(DMatrix::from_row_slice(1, 2, &[11_i8, 12]));
        let function = MathAbsI8MD {
            arg: arg.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(&*out.borrow(), &DMatrix::from_row_slice(1, 2, &[2, 3]));
        *arg.borrow_mut() = DMatrix::from_row_slice(1, 2, &[-4, i8::MIN]);

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(&*out.borrow(), &DMatrix::from_row_slice(1, 2, &[2, 3]));
    }
}

#[cfg(feature = "source")]
fn impl_abs_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms2!(
      MathAbs,
      (lhs_value),
      U8 => MatrixU8, u8, u8::zero(), "u8";
      U16 => MatrixU16, u16, u16::zero(), "u16";
      U32 => MatrixU32, u32, u32::zero(), "u32";
      U64 => MatrixU64, u64, u64::zero(), "u64";
      U128 => MatrixU128, u128, u128::zero(), "u128";
      I8 => MatrixI8, i8, i8::zero(), "i8";
      I16 => MatrixI16, i16, i16::zero(), "i16";
      I32 => MatrixI32, i32, i32::zero(), "i32";
      I64 => MatrixI64, i64, i64::zero(), "i64";
      I128 => MatrixI128, i128, i128::zero(), "i128";
      F32 => MatrixF32, f32, f32::zero(), "f32";
      F64 => MatrixF64, f64, f64::zero(), "f64";
      C64 => MatrixC64, C64, C64::default(), "c64";
      R64 => MatrixR64, R64, R64::zero(), "r64";
    )
}

#[cfg(feature = "source")]
pub struct MathAbs {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAbs {
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
        match impl_abs_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (input) {
                (Value::MutableReference(input)) => impl_abs_fxn(input.borrow().clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "math/abs".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
