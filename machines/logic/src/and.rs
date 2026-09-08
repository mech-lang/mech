use crate::*;

// And ------------------------------------------------------------------------

macro_rules! and_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::Exact, |lhs, rhs| {
            lhs && rhs
        })
    };
}

macro_rules! and_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        and_op!($lhs, $rhs, $out)
    };
}

macro_rules! and_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftScalar, |lhs, rhs| {
            lhs && rhs
        })
    };
}

macro_rules! and_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightScalar, |lhs, rhs| {
            lhs && rhs
        })
    };
}

macro_rules! and_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightColumn, |lhs, rhs| {
            lhs && rhs
        })
    };
}

macro_rules! and_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftColumn, |lhs, rhs| {
            lhs && rhs
        })
    };
}

macro_rules! and_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightRow, |lhs, rhs| {
            lhs && rhs
        })
    };
}

macro_rules! and_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftRow, |lhs, rhs| {
            lhs && rhs
        })
    };
}

impl_logic_fxns!(And);

impl_canonical_logic_binop_specializer!(LogicAnd, and, And, "logic/and");
