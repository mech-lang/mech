use crate::*;

// Greater Than ---------------------------------------------------------------

macro_rules! gt_scalar_lhs_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightScalar, |lhs, rhs| lhs > rhs) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightScalar,
            |lhs, rhs| lhs > rhs,
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i] > (*$rhs);
            }
        }
    };
}

macro_rules! gt_scalar_rhs_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftScalar, |lhs, rhs| lhs > rhs) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftScalar,
            |lhs, rhs| lhs > rhs,
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$rhs).len() {
                (&mut (*$out))[i] = (*$lhs) > (&(*$rhs))[i];
            }
        }
    };
}

macro_rules! gt_vec_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| lhs > rhs) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison($lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| {
            lhs > rhs
        })
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i] > (&(*$rhs))[i];
            }
        }
    };
}

macro_rules! gt_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| lhs > rhs) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison($lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| {
            lhs > rhs
        })
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$lhs) > (*$rhs);
        }
    };
}

macro_rules! gt_mat_vec_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightColumn, |lhs, rhs| lhs > rhs) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightColumn,
            |lhs, rhs| lhs > rhs,
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_col[i] > rhs_deref[i];
                }
            }
        }
    };
}

macro_rules! gt_vec_mat_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftColumn, |lhs, rhs| lhs > rhs) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftColumn,
            |lhs, rhs| lhs > rhs,
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_deref[i] > rhs_col[i];
                }
            }
        }
    };
}

macro_rules! gt_mat_row_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightRow, |lhs, rhs| lhs > rhs) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightRow,
            |lhs, rhs| lhs > rhs,
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_row[i] > rhs_deref[i];
                }
            }
        }
    };
}

macro_rules! gt_row_mat_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftRow, |lhs, rhs| lhs > rhs) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftRow,
            |lhs, rhs| lhs > rhs,
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_deref[i] > rhs_row[i];
                }
            }
        }
    };
}

impl_compare_fxns!(GT);

impl_canonical_numeric_compare_specializer!(CompareGreaterThan, gt, GT, "compare/gt");
