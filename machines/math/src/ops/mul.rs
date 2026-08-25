use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

fn checked_runtime_mul<T: RuntimeCheckedArithmetic>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_mul(rhs)
        .ok_or_else(|| arithmetic_overflow::<T>("multiplication"))
}

// Mul ------------------------------------------------------------------------

macro_rules! mul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_mul(*$lhs, *$rhs)?;
            *$out = next;
        }
    };
}

macro_rules! mul_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (o, (l, r)) in next
                .iter_mut()
                .zip(lhs_deref.iter().zip(rhs_deref.iter()))
            {
                *o = checked_runtime_mul(*l, *r)?;
            }
            *$out = next;
        }
    };
}

macro_rules! mul_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = (*$rhs);
            for (o, l) in next.iter_mut().zip(lhs_deref.iter()) {
                *o = checked_runtime_mul(*l, rhs_deref)?;
            }
            *$out = next;
        }
    };
}

macro_rules! mul_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = (*$lhs);
            let rhs_deref = &(*$rhs);
            for (o, r) in next.iter_mut().zip(rhs_deref.iter()) {
                *o = checked_runtime_mul(lhs_deref, *r)?;
            }
            *$out = next;
        }
    };
}

macro_rules! mul_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in next.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_mul(lhs_col[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! mul_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in next.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_mul(lhs_deref[i], rhs_col[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! mul_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in next.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_mul(lhs_row[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! mul_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in next.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_mul(lhs_deref[i], rhs_row[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! impl_checked_mul_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_checked_arithmetic_binop!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            $op,
            crate::ops::arithmetic_full_write_contract
        );
    };
}

impl_fxns!(Mul, T, T, impl_checked_mul_binop);

#[cfg(all(test, feature = "u8"))]
mod checked_arithmetic_tests {
    use super::*;

    #[test]
    fn integer_multiplication_rejects_reactive_overflow_and_retains_output() {
        let rhs = Ref::new(2_u8);
        let out = Ref::new(17_u8);
        let function = MulSS {
            lhs: Ref::new(20_u8),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 40);
        *rhs.borrow_mut() = 20;
        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(*out.borrow(), 40);
    }
}

#[cfg(feature = "source")]
fn impl_mul_fxn(lhs_value: LegacyValue, rhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_binop_match_arms!(
      Mul,
      (lhs_value, rhs_value),
      I8,   i8,   "i8";
      I16,  i16,  "i16";
      I32,  i32,  "i32";
      I64,  i64,  "i64";
      I128, i128, "i128";
      U8,   u8,   "u8";
      U16,  u16,  "u16";
      U32,  u32,  "u32";
      U64,  u64,  "u64";
      U128, u128, "u128";
      F32,  f32,  "f32";
      F64,  f64,  "f64";
      R64, R64, "rational";
      C64, C64, "complex";
    )
}

#[cfg(feature = "source")]
impl_mech_binop_fxn!(MathMul, impl_mul_fxn, "math/mul");
