//! Shared semantic vocabulary. Operation identities alone grant no binder witness.
/// Exact semantic kinds supported by the canonical scalar native adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeScalarKind {
    Bool,
    U64,
    I64,
    F64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32,
}

#[cfg(feature = "native-artifact")]
impl NativeScalarKind {
    /// Maximum canonical unsigned payload; other kinds preserve their own bits.
    pub const fn unsigned_maximum(self) -> Option<u64> {
        match self {
            Self::U8 => Some(u8::MAX as u64),
            Self::U16 => Some(u16::MAX as u64),
            Self::U32 => Some(u32::MAX as u64),
            Self::U64 => Some(u64::MAX),
            _ => None,
        }
    }
    /// A raw word must fit the exact semantic type, even in an eight-byte lane.
    pub const fn accepts_bits(self, bits: u64) -> bool {
        match self {
            Self::Bool => bits <= 1,
            Self::U8 => bits <= u8::MAX as u64,
            Self::U16 => bits <= u16::MAX as u64,
            Self::U32 => bits <= u32::MAX as u64,
            Self::U64 | Self::I64 | Self::F64 => true,
        }
    }
}

/// Checked operations certified by the actual selected maintained binder.
/// These are semantic identities, not source names or catalog hashes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeScalarOp {
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64BitAnd,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64BitOr,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64BitXor,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64BitNot,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64ShlWrap,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64ShrWrap,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64Clz,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64Ctz,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64WrappingAdd,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64WrappingSub,
    #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
    U64WrappingMul,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32Add,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32Sub,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8Add,
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
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8ToU16,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8ToU32,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8ToU64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16ToU8,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16ToU32,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16ToU64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32ToU8,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32ToU16,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32ToU64,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U64ToU8,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U64ToU16,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U64ToU32,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8Eq,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8Ne,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8Lt,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8Le,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8Gt,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U8Ge,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16Eq,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16Ne,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16Lt,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16Le,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16Gt,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U16Ge,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32Eq,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32Ne,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32Lt,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32Le,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32Gt,
    #[cfg(any(feature = "convert", feature = "native-artifact"))]
    U32Ge,
}

impl NativeScalarOp {
    pub const fn signature(self) -> (&'static [NativeScalarKind], NativeScalarKind) {
        use NativeScalarKind::*;
        use NativeScalarOp::*;
        match self {
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U8ToU16 => (&[U8], U16),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U8ToU32 => (&[U8], U32),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U8ToU64 => (&[U8], U64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U16ToU8 => (&[U16], U8),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U16ToU32 => (&[U16], U32),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U16ToU64 => (&[U16], U64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U32ToU8 => (&[U32], U8),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U32ToU16 => (&[U32], U16),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U32ToU64 => (&[U32], U64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U64ToU8 => (&[U64], U8),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U64ToU16 => (&[U64], U16),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U64ToU32 => (&[U64], U32),
            #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
            U64BitNot | U64Clz | U64Ctz => (&[U64], U64),
            #[cfg(any(feature = "integer_bits", feature = "native-artifact"))]
            U64BitAnd | U64BitOr | U64BitXor | U64ShlWrap | U64ShrWrap | U64WrappingAdd
            | U64WrappingSub | U64WrappingMul => (&[U64, U64], U64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U32Add | U32Sub => (&[U32, U32], U32),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U8Add => (&[U8, U8], U8),
            U64Add | U64Sub | U64Mul | U64Div | U64Rem => (&[U64, U64], U64),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U8Eq | U8Ne | U8Lt | U8Le | U8Gt | U8Ge => (&[U8, U8], Bool),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U16Eq | U16Ne | U16Lt | U16Le | U16Gt | U16Ge => (&[U16, U16], Bool),
            #[cfg(any(feature = "convert", feature = "native-artifact"))]
            U32Eq | U32Ne | U32Lt | U32Le | U32Gt | U32Ge => (&[U32, U32], Bool),
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
