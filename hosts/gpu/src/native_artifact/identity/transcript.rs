//! Shared frozen scalar identity grammar. Family owners choose the transcript
//! order and retain independent plan, image and same-Object validation.
use mech_engine::native_artifact::{
    NativeMatchAction, NativeMatchConstant, NativeMatchInput, NativeMatchOrigin,
    NativeMatchTerminator, NativeScalarKind, NativeScalarType, NativeScalarValue,
};
use sha2::{Digest, Sha256};

#[inline(always)]
pub(in crate::native_artifact) fn word(hash: &mut Sha256, value: u64) {
    hash.update(value.to_le_bytes());
}

#[inline(always)]
pub(in crate::native_artifact) fn bytes(hash: &mut Sha256, value: &[u8]) {
    word(hash, value.len() as u64);
    hash.update(value);
}

pub(in crate::native_artifact) fn scalar_kind(kind: NativeScalarKind) -> u8 {
    match kind {
        NativeScalarKind::Bool => 1,
        NativeScalarKind::U64 => 2,
        NativeScalarKind::I64 => 3,
        NativeScalarKind::F64 => 4,
        NativeScalarKind::U8 => 5,
        NativeScalarKind::U16 => 6,
        NativeScalarKind::U32 => 7,
    }
}

pub(in crate::native_artifact) fn scalar_type(hash: &mut Sha256, value: &NativeScalarType) {
    hash.update([scalar_kind(value.kind())]);
    word(hash, u64::from(value.schema().get()));
    hash.update(value.schema_key().as_bytes());
}

pub(in crate::native_artifact) fn scalar_value(hash: &mut Sha256, value: NativeScalarValue) {
    let (tag, bits) = match value {
        NativeScalarValue::Bool(value) => (1, u64::from(value)),
        NativeScalarValue::U64(value) => (2, value),
        NativeScalarValue::I64(value) => (3, value as u64),
        NativeScalarValue::F64(value) => (4, value.bits()),
        NativeScalarValue::U8(value) => (5, u64::from(value)),
        NativeScalarValue::U16(value) => (6, u64::from(value)),
        NativeScalarValue::U32(value) => (7, u64::from(value)),
    };
    hash.update([tag]);
    word(hash, bits);
}

pub(in crate::native_artifact) fn scalar_bindings(
    hash: &mut Sha256,
    values: &[NativeScalarType],
    inputs: &[NativeMatchInput],
    constants: &[NativeMatchConstant],
) {
    word(hash, values.len() as u64);
    for value in values {
        scalar_type(hash, value);
    }
    word(hash, inputs.len() as u64);
    for input in inputs {
        word(hash, u64::from(input.declaration().input().get()));
        word(hash, u64::from(input.declaration().slot().get()));
        bytes(hash, input.declaration().name().as_bytes());
        word(hash, u64::from(input.value().get()));
    }
    word(hash, constants.len() as u64);
    for constant in constants {
        word(hash, u64::from(constant.value().get()));
        scalar_value(hash, constant.scalar());
    }
}

pub(in crate::native_artifact) fn optional_index(hash: &mut Sha256, value: Option<u32>) {
    match value {
        Some(value) => {
            hash.update([1]);
            word(hash, u64::from(value));
        }
        None => hash.update([0]),
    }
}

pub(in crate::native_artifact) fn origin(hash: &mut Sha256, value: NativeMatchOrigin) {
    word(hash, u64::from(value.node().get()));
    optional_index(hash, value.block());
    optional_index(hash, value.local());
}

pub(in crate::native_artifact) fn scalar_action(hash: &mut Sha256, action: &NativeMatchAction) {
    match action {
        NativeMatchAction::None => hash.update([0]),
        NativeMatchAction::Primitive {
            op,
            operands,
            output,
            ..
        } => {
            hash.update([1, super::op_tag(*op)]);
            word(hash, u64::from(output.get()));
            word(hash, operands.len() as u64);
            for operand in operands {
                word(hash, u64::from(operand.get()));
            }
        }
        NativeMatchAction::Copy { source, output } => {
            hash.update([2]);
            word(hash, u64::from(source.get()));
            word(hash, u64::from(output.get()));
        }
    }
}

pub(in crate::native_artifact) fn scalar_terminator(
    hash: &mut Sha256,
    terminator: NativeMatchTerminator,
) {
    match terminator {
        NativeMatchTerminator::Jump(next) => {
            hash.update([1]);
            word(hash, u64::from(next));
        }
        NativeMatchTerminator::Branch { condition, yes, no } => {
            hash.update([2]);
            word(hash, u64::from(condition.get()));
            word(hash, u64::from(yes));
            word(hash, u64::from(no));
        }
        NativeMatchTerminator::LiteralBranch {
            left,
            right,
            yes,
            no,
        } => {
            hash.update([3]);
            for value in [left.get(), right.get(), yes, no] {
                word(hash, u64::from(value));
            }
        }
        NativeMatchTerminator::Complete => hash.update([4]),
        NativeMatchTerminator::Reject => hash.update([5]),
        NativeMatchTerminator::IntegrityFailure => hash.update([6]),
    }
}
