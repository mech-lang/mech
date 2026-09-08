use crate::*;

// Xor ------------------------------------------------------------------------

macro_rules! xor_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::Exact, |lhs, rhs| {
            lhs ^ rhs
        })
    };
}

macro_rules! xor_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        xor_op!($lhs, $rhs, $out)
    };
}

macro_rules! xor_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftScalar, |lhs, rhs| {
            lhs ^ rhs
        })
    };
}

macro_rules! xor_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightScalar, |lhs, rhs| {
            lhs ^ rhs
        })
    };
}

macro_rules! xor_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightColumn, |lhs, rhs| {
            lhs ^ rhs
        })
    };
}

macro_rules! xor_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftColumn, |lhs, rhs| {
            lhs ^ rhs
        })
    };
}

macro_rules! xor_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::RightRow, |lhs, rhs| {
            lhs ^ rhs
        })
    };
}

macro_rules! xor_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        apply_logic_binary($lhs, $rhs, $out, LogicBroadcast::LeftRow, |lhs, rhs| {
            lhs ^ rhs
        })
    };
}

impl_logic_fxns!(Xor);

impl_canonical_logic_binop_specializer!(LogicXor, xor, Xor, "logic/xor");
