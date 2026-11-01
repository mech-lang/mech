//! Inputs only: actual maintained core/resident execution supplies result oracles.
use mech_core::snapshot::F64Bits;
use mech_engine::native_artifact::{NativeScalarKind, NativeScalarOp, NativeScalarValue};
#[derive(Clone, Copy)]
pub struct Cast {
    pub name: &'static str,
    pub source: &'static str,
    pub op: NativeScalarOp,
    pub input: NativeScalarKind,
    pub output: NativeScalarKind,
}
pub const CASTS: [Cast; 6] = [
    Cast {
        name: "canonical-linked-cast-u64-i64",
        source: "result<i64> := value<u64>\nresult\n",
        op: NativeScalarOp::U64ToI64,
        input: NativeScalarKind::U64,
        output: NativeScalarKind::I64,
    },
    Cast {
        name: "canonical-linked-cast-i64-u64",
        source: "result<u64> := value<i64>\nresult\n",
        op: NativeScalarOp::I64ToU64,
        input: NativeScalarKind::I64,
        output: NativeScalarKind::U64,
    },
    Cast {
        name: "canonical-linked-cast-u64-f64",
        source: "result<f64> := value<u64>\nresult\n",
        op: NativeScalarOp::U64ToF64,
        input: NativeScalarKind::U64,
        output: NativeScalarKind::F64,
    },
    Cast {
        name: "canonical-linked-cast-i64-f64",
        source: "result<f64> := value<i64>\nresult\n",
        op: NativeScalarOp::I64ToF64,
        input: NativeScalarKind::I64,
        output: NativeScalarKind::F64,
    },
    Cast {
        name: "canonical-linked-cast-f64-u64",
        source: "result<u64> := value<f64>\nresult\n",
        op: NativeScalarOp::F64ToU64,
        input: NativeScalarKind::F64,
        output: NativeScalarKind::U64,
    },
    Cast {
        name: "canonical-linked-cast-f64-i64",
        source: "result<i64> := value<f64>\nresult\n",
        op: NativeScalarOp::F64ToI64,
        input: NativeScalarKind::F64,
        output: NativeScalarKind::I64,
    },
];
pub fn samples(cast: Cast) -> Vec<NativeScalarValue> {
    match cast.input {
        NativeScalarKind::U64 => [
            0,
            1,
            2,
            (1u64 << 53) - 1,
            1u64 << 53,
            (1u64 << 53) + 1,
            (1u64 << 53) + 2,
            (1u64 << 53) + 3,
            (1u64 << 54) - 1,
            (1u64 << 54) + 1,
            (1u64 << 54) + 2,
            (1u64 << 54) + 3,
            i64::MAX as u64 - 1024,
            i64::MAX as u64,
            i64::MAX as u64 + 1,
            (1u64 << 63) + 1025,
            u64::MAX - 1024,
            u64::MAX,
        ]
        .into_iter()
        .map(NativeScalarValue::U64)
        .collect(),
        NativeScalarKind::I64 => [
            i64::MIN,
            i64::MIN + 1,
            i64::MIN + 1025,
            -((1i64 << 54) + 3),
            -((1i64 << 53) + 3),
            -((1i64 << 53) + 2),
            -((1i64 << 53) + 1),
            -(1i64 << 53),
            -1,
            0,
            1,
            (1i64 << 53) - 1,
            1i64 << 53,
            (1i64 << 53) + 1,
            (1i64 << 53) + 2,
            (1i64 << 53) + 3,
            (1i64 << 54) + 3,
            i64::MAX - 1024,
            i64::MAX,
        ]
        .into_iter()
        .map(NativeScalarValue::I64)
        .collect(),
        NativeScalarKind::F64 => [
            0x0000_0000_0000_0000,
            0x8000_0000_0000_0000,
            0x0000_0000_0000_0001,
            0x8000_0000_0000_0001,
            0x000f_ffff_ffff_ffff,
            0x800f_ffff_ffff_ffff,
            0x0010_0000_0000_0000,
            0x8010_0000_0000_0000,
            0x3fe0_0000_0000_0000,
            0xbfe0_0000_0000_0000,
            0xbfee_ffff_ffff_ffff,
            0xbfef_ffff_ffff_ffff,
            0xbff0_0000_0000_0000,
            0xbff0_0000_0000_0001,
            0x3ff0_0000_0000_0000,
            0x3ffe_6666_6666_6666,
            0xbffe_6666_6666_6666,
            0x433f_ffff_ffff_ffff,
            0x4340_0000_0000_0000,
            0x4340_0000_0000_0001,
            0x43df_ffff_ffff_ffff,
            0x43e0_0000_0000_0000,
            0x43e0_0000_0000_0001,
            0xc3df_ffff_ffff_ffff,
            0xc3e0_0000_0000_0000,
            0xc3e0_0000_0000_0001,
            0x43ef_ffff_ffff_ffff,
            0x43f0_0000_0000_0000,
            0x43f0_0000_0000_0001,
            0x7fef_ffff_ffff_ffff,
            0xffef_ffff_ffff_ffff,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0x7ff8_0000_0000_0001,
            0xfff8_0000_0000_1234,
            0x7ff0_0000_0000_0001,
            0xfff0_0000_0000_4321,
        ]
        .into_iter()
        .map(|bits| NativeScalarValue::F64(F64Bits::from_bits(bits)))
        .collect(),
        NativeScalarKind::Bool => unreachable!("cast inputs are numeric"),
    }
}
