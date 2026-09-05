use crate::*;

// Less Than Equal ---------------------------------------------------------------

macro_rules! lte_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$lhs).len() {
                (&mut *$out)[i] = (&*$lhs)[i] <= *$rhs;
            }
        }
    };
}

macro_rules! lte_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$rhs).len() {
                (&mut *$out)[i] = *$lhs <= (&*$rhs)[i];
            }
        }
    };
}

macro_rules! lte_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$lhs).len() {
                (&mut *$out)[i] = (&*$lhs)[i] <= (&*$rhs)[i];
            }
        }
    };
}

macro_rules! lte_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$lhs) <= (*$rhs);
        }
    };
}

macro_rules! lte_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_col[i] <= rhs_deref[i];
                }
            }
        }
    };
}

macro_rules! lte_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_deref[i] <= rhs_col[i];
                }
            }
        }
    };
}

macro_rules! lte_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_row[i] <= rhs_deref[i];
                }
            }
        }
    };
}

macro_rules! lte_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_deref[i] <= rhs_row[i];
                }
            }
        }
    };
}

impl_compare_fxns!(LTE);

impl_canonical_numeric_compare_specializer!(CompareLessThanEqual, lte, LTE, "compare/lte");
