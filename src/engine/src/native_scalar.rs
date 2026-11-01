//! Shared semantic vocabulary. Operation identities alone grant no binder witness.
/// Exact semantic kinds supported by the canonical scalar native adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeScalarKind {
    Bool,
    U64,
    I64,
    F64,
}

/// Checked operations certified by the actual selected maintained binder.
/// These are semantic identities, not source names or catalog hashes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeScalarOp {
    U64Add,
    U64Sub,
    U64Mul,
    U64Div,
    U64Rem,
    U64Eq,
    U64Ne,
    U64Lt,
    U64Le,
    U64Gt,
    U64Ge,
    BoolAnd,
    BoolOr,
    BoolXor,
    BoolNot,
    BoolEq,
    BoolNe,
    I64Add,
    I64Sub,
    I64Mul,
    I64Div,
    I64Rem,
    I64Neg,
    I64Eq,
    I64Ne,
    I64Lt,
    I64Le,
    I64Gt,
    I64Ge,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Neg,
    F64Sqrt,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Le,
    F64Gt,
    F64Ge,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U64ToI64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    I64ToU64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U64ToF64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    I64ToF64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    F64ToU64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    F64ToI64,
}

impl NativeScalarOp {
    pub const fn signature(self) -> (&'static [NativeScalarKind], NativeScalarKind) {
        use NativeScalarKind::{Bool, F64, I64, U64};
        use NativeScalarOp::*;
        match self {
            U64Add | U64Sub | U64Mul | U64Div | U64Rem => (&[U64, U64], U64),
            U64Eq | U64Ne | U64Lt | U64Le | U64Gt | U64Ge => (&[U64, U64], Bool),
            I64Add | I64Sub | I64Mul | I64Div | I64Rem => (&[I64, I64], I64),
            I64Eq | I64Ne | I64Lt | I64Le | I64Gt | I64Ge => (&[I64, I64], Bool),
            I64Neg => (&[I64], I64),
            F64Add | F64Sub | F64Mul | F64Div => (&[F64, F64], F64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U64ToI64 => (&[U64], I64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            I64ToU64 => (&[I64], U64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U64ToF64 => (&[U64], F64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            I64ToF64 => (&[I64], F64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            F64ToU64 => (&[F64], U64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            F64ToI64 => (&[F64], I64),
            F64Neg | F64Sqrt => (&[F64], F64),
            F64Eq | F64Ne | F64Lt | F64Le | F64Gt | F64Ge => (&[F64, F64], Bool),
            BoolNot => (&[Bool], Bool),
            BoolAnd | BoolOr | BoolXor | BoolEq | BoolNe => (&[Bool, Bool], Bool),
        }
    }
}
