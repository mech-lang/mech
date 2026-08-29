use crate::*;

// Min ------------------------------------------------------------------------

#[cfg(feature = "matrix")]
macro_rules! min_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$lhs).len() {
                let a = (&*$lhs)[i].clone();
                let b = (*$rhs).clone();
                (&mut *$out)[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) {
                    a
                } else {
                    b
                };
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! min_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$rhs).len() {
                let a = (*$lhs).clone();
                let b = (&*$rhs)[i].clone();
                (&mut *$out)[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) {
                    a
                } else {
                    b
                };
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! min_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$lhs).len() {
                let a = (&*$lhs)[i].clone();
                let b = (&*$rhs)[i].clone();
                (&mut *$out)[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) {
                    a
                } else {
                    b
                };
            }
        }
    };
}

macro_rules! min_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let a = (*$lhs).clone();
            let b = (*$rhs).clone();
            (*$out) = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) {
                a
            } else {
                b
            };
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! min_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    let a = lhs_col[i].clone();
                    let b = rhs_deref[i].clone();
                    col[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) {
                        a
                    } else {
                        b
                    };
                }
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! min_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    let a = lhs_deref[i].clone();
                    let b = rhs_col[i].clone();
                    col[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) {
                        a
                    } else {
                        b
                    };
                }
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! min_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    let a = lhs_row[i].clone();
                    let b = rhs_deref[i].clone();
                    row[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) {
                        a
                    } else {
                        b
                    };
                }
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! min_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    let a = lhs_deref[i].clone();
                    let b = rhs_row[i].clone();
                    row[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) {
                        a
                    } else {
                        b
                    };
                }
            }
        }
    };
}

impl_compare_fxns2!(Min);

impl_canonical_numeric_compare_specializer!(CompareMin, min, Min, "compare/min");
