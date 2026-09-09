use crate::*;

// Or ------------------------------------------------------------------------

macro_rules! or_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::Exact, |lhs, rhs| {
            lhs || rhs
        })
    };
}

#[cfg(feature = "matrix")]
macro_rules! or_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        or_op!($lhs, $rhs, $out)
    };
}

#[cfg(feature = "matrix")]
macro_rules! or_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftScalar, |lhs, rhs| {
            lhs || rhs
        })
    };
}

#[cfg(feature = "matrix")]
macro_rules! or_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightScalar, |lhs, rhs| {
            lhs || rhs
        })
    };
}

#[cfg(feature = "matrix")]
macro_rules! or_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightColumn, |lhs, rhs| {
            lhs || rhs
        })
    };
}

#[cfg(feature = "matrix")]
macro_rules! or_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftColumn, |lhs, rhs| {
            lhs || rhs
        })
    };
}

#[cfg(feature = "matrix")]
macro_rules! or_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightRow, |lhs, rhs| {
            lhs || rhs
        })
    };
}

#[cfg(feature = "matrix")]
macro_rules! or_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftRow, |lhs, rhs| {
            lhs || rhs
        })
    };
}

impl_logic_fxns!(Or);

impl_canonical_logic_binop_specializer!(LogicOr, or, Or, "logic/or");
