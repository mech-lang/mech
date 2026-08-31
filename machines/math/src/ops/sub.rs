use crate::*;
use num_traits::*;

fn checked_runtime_sub<T: RuntimeCheckedArithmetic>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_sub(rhs)
        .ok_or_else(|| arithmetic_overflow::<T>("subtraction"))
}

// Sub ------------------------------------------------------------------------

macro_rules! sub_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_sub(*$lhs, *$rhs)?;
            *$out = next;
        }
    };
}

macro_rules! sub_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for (output, (lhs, rhs)) in next.iter_mut().zip((*$lhs).iter().zip((*$rhs).iter())) {
                *output = checked_runtime_sub(*lhs, *rhs)?;
            }
            *$out = next;
        }
    };
}

macro_rules! sub_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for i in 0..(&*$lhs).len() {
                next[i] = checked_runtime_sub((&*$lhs)[i], *$rhs)?;
            }
            *$out = next;
        }
    };
}

macro_rules! sub_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for i in 0..(&*$rhs).len() {
                next[i] = checked_runtime_sub(*$lhs, (&*$rhs)[i])?;
            }
            *$out = next;
        }
    };
}

macro_rules! sub_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in next.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_sub(lhs_col[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! sub_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in next.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_sub(lhs_deref[i], rhs_col[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! sub_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in next.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_sub(lhs_row[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! sub_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in next.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_sub(lhs_deref[i], rhs_row[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! impl_checked_sub_binop {
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

impl_fxns!(Sub, T, T, impl_checked_sub_binop);

#[cfg(all(test, feature = "u8"))]
mod checked_arithmetic_tests {
    use super::*;

    #[test]
    fn integer_subtraction_rejects_reactive_overflow_and_retains_output() {
        let rhs = Ref::new(1_u8);
        let out = Ref::new(17_u8);
        let function = SubSS {
            lhs: Ref::new(40_u8),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 39);
        *rhs.borrow_mut() = 41;
        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(*out.borrow(), 39);
    }
}

impl_canonical_registered_math_binop_specializer!(MathSub, "Sub");
