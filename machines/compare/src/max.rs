use crate::*;

// Max ------------------------------------------------------------------------

macro_rules! max_scalar_lhs_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightScalar, |lhs, rhs| if lhs >= rhs { lhs.to_owned() } else { rhs.to_owned() }) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightScalar,
            |lhs, rhs| {
                if lhs.partial_cmp(&rhs) != Some(std::cmp::Ordering::Less) {
                    lhs
                } else {
                    rhs
                }
            },
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$lhs).len() {
                let a = (&*$lhs)[i].clone();
                let b = (*$rhs).clone();
                (&mut *$out)[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
                    a
                } else {
                    b
                };
            }
        }
    };
}

macro_rules! max_scalar_rhs_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftScalar, |lhs, rhs| if lhs >= rhs { lhs.to_owned() } else { rhs.to_owned() }) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftScalar,
            |lhs, rhs| {
                if lhs.partial_cmp(&rhs) != Some(std::cmp::Ordering::Less) {
                    lhs
                } else {
                    rhs
                }
            },
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$rhs).len() {
                let a = (*$lhs).clone();
                let b = (&*$rhs)[i].clone();
                (&mut *$out)[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
                    a
                } else {
                    b
                };
            }
        }
    };
}

macro_rules! max_vec_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| if lhs >= rhs { lhs.to_owned() } else { rhs.to_owned() }) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison($lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| {
            if lhs.partial_cmp(&rhs) != Some(std::cmp::Ordering::Less) {
                lhs
            } else {
                rhs
            }
        })
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(&*$lhs).len() {
                let a = (&*$lhs)[i].clone();
                let b = (&*$rhs)[i].clone();
                (&mut *$out)[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
                    a
                } else {
                    b
                };
            }
        }
    };
}

macro_rules! max_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| if lhs >= rhs { lhs.to_owned() } else { rhs.to_owned() }) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison($lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| {
            if lhs.partial_cmp(&rhs) != Some(std::cmp::Ordering::Less) {
                lhs
            } else {
                rhs
            }
        })
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let a = (*$lhs).clone();
            let b = (*$rhs).clone();
            (*$out) = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
                a
            } else {
                b
            };
        }
    };
}

macro_rules! max_mat_vec_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightColumn, |lhs, rhs| if lhs >= rhs { lhs.to_owned() } else { rhs.to_owned() }) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightColumn,
            |lhs, rhs| {
                if lhs.partial_cmp(&rhs) != Some(std::cmp::Ordering::Less) {
                    lhs
                } else {
                    rhs
                }
            },
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    let a = lhs_col[i].clone();
                    let b = rhs_deref[i].clone();
                    col[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
                        a
                    } else {
                        b
                    };
                }
            }
        }
    };
}

macro_rules! max_vec_mat_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftColumn, |lhs, rhs| if lhs >= rhs { lhs.to_owned() } else { rhs.to_owned() }) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftColumn,
            |lhs, rhs| {
                if lhs.partial_cmp(&rhs) != Some(std::cmp::Ordering::Less) {
                    lhs
                } else {
                    rhs
                }
            },
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    let a = lhs_deref[i].clone();
                    let b = rhs_col[i].clone();
                    col[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
                        a
                    } else {
                        b
                    };
                }
            }
        }
    };
}

macro_rules! max_mat_row_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightRow, |lhs, rhs| if lhs >= rhs { lhs.to_owned() } else { rhs.to_owned() }) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightRow,
            |lhs, rhs| {
                if lhs.partial_cmp(&rhs) != Some(std::cmp::Ordering::Less) {
                    lhs
                } else {
                    rhs
                }
            },
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    let a = lhs_row[i].clone();
                    let b = rhs_deref[i].clone();
                    row[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
                        a
                    } else {
                        b
                    };
                }
            }
        }
    };
}

macro_rules! max_row_mat_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => { apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftRow, |lhs, rhs| if lhs >= rhs { lhs.to_owned() } else { rhs.to_owned() }) };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftRow,
            |lhs, rhs| {
                if lhs.partial_cmp(&rhs) != Some(std::cmp::Ordering::Less) {
                    lhs
                } else {
                    rhs
                }
            },
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    let a = lhs_deref[i].clone();
                    let b = rhs_row[i].clone();
                    row[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
                        a
                    } else {
                        b
                    };
                }
            }
        }
    };
}

impl_compare_fxns2!(Max);

impl_canonical_numeric_compare_specializer!(CompareMax, max, Max, "compare/max");
