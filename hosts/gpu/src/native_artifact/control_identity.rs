//! Explicit versioned executable CFG identity; no Debug/serde encoding.
use mech_engine::native_artifact::{
    NativeMatchAction, NativeMatchArtifactPlan, NativeMatchTerminator, NativeScalarKind,
    NativeScalarValue,
};
use sha2::{Digest, Sha256};
fn word(hash: &mut Sha256, value: u64) {
    hash.update(value.to_le_bytes());
}
fn bytes(hash: &mut Sha256, value: &[u8]) {
    word(hash, value.len() as u64);
    hash.update(value);
}
pub(super) fn object_record(
    plan: &NativeMatchArtifactPlan,
    descriptor: &super::control_abi::NativeMatchDescriptor,
) -> Result<[u8; 168], super::NativeCompileError> {
    let mut bytes = [0; 168];
    bytes[..16].copy_from_slice(b"MECHNATIVEMATCH\0");
    for (at, value) in [
        (16, 2u64),
        (24, super::control_abi::VERSION),
        (32, 1),
        (112, descriptor.stack_bytes()),
        (120, descriptor.code_bytes()),
        (128, plan.inputs().len() as u64),
        (136, plan.values().len() as u64),
        (144, plan.blocks().len() as u64),
        (152, plan.outputs().len() as u64),
        (160, plan.entry() as u64),
    ] {
        bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }
    bytes[40..72].copy_from_slice(plan.program_revision().as_bytes());
    bytes[72..104].copy_from_slice(descriptor.typed_ir_sha256());
    bytes[104..112].copy_from_slice(
        &super::identity::host_target_tag().ok_or(super::NativeCompileError::UnsupportedHost)?,
    );
    Ok(bytes)
}
pub(super) fn typed_ir_sha256(plan: &NativeMatchArtifactPlan) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"mech.canonical-scalar-match-cfg\0");
    word(&mut hash, 2); // Wider scalar/literal semantics; separate from scalar IR.
    word(&mut hash, super::control_abi::VERSION);
    word(&mut hash, 1); // Control visit work policy.
    hash.update(plan.program_revision().as_bytes());
    word(&mut hash, plan.entry() as u64);
    word(&mut hash, plan.values().len() as u64);
    for ty in plan.values() {
        hash.update([match ty.kind() {
            NativeScalarKind::Bool => 1,
            NativeScalarKind::U64 => 2,
            NativeScalarKind::I64 => 3,
            NativeScalarKind::F64 => 4,
        }]);
        word(&mut hash, ty.schema().get() as u64);
        hash.update(ty.schema_key().as_bytes());
    }
    word(&mut hash, plan.inputs().len() as u64);
    for input in plan.inputs() {
        word(&mut hash, input.declaration().input().get() as u64);
        word(&mut hash, input.declaration().slot().get() as u64);
        bytes(&mut hash, input.declaration().name().as_bytes());
        word(&mut hash, input.value().get() as u64);
    }
    word(&mut hash, plan.constants().len() as u64);
    for constant in plan.constants() {
        word(&mut hash, constant.value().get() as u64);
        match constant.scalar() {
            NativeScalarValue::Bool(value) => {
                hash.update([1]);
                word(&mut hash, value as u64);
            }
            NativeScalarValue::U64(value) => {
                hash.update([2]);
                word(&mut hash, value);
            }
            NativeScalarValue::I64(value) => {
                hash.update([3]);
                word(&mut hash, value as u64);
            }
            NativeScalarValue::F64(value) => {
                hash.update([4]);
                word(&mut hash, value.bits());
            }
        }
    }
    word(&mut hash, plan.blocks().len() as u64);
    for block in plan.blocks() {
        let at = block.origin();
        word(&mut hash, at.node().get() as u64);
        for id in [at.block(), at.local()] {
            match id {
                Some(id) => {
                    hash.update([1]);
                    word(&mut hash, id as u64);
                }
                None => hash.update([0]),
            }
        }
        match block.action() {
            NativeMatchAction::None => hash.update([0]),
            NativeMatchAction::Primitive {
                op,
                operands,
                output,
                ..
            } => {
                hash.update([1, super::identity::op_tag(*op)]);
                word(&mut hash, output.get() as u64);
                word(&mut hash, operands.len() as u64);
                for operand in operands {
                    word(&mut hash, operand.get() as u64);
                }
            }
            NativeMatchAction::Copy { source, output } => {
                hash.update([2]);
                word(&mut hash, source.get() as u64);
                word(&mut hash, output.get() as u64);
            }
        }
        match block.terminator() {
            NativeMatchTerminator::Jump(next) => {
                hash.update([1]);
                word(&mut hash, next as u64);
            }
            NativeMatchTerminator::Branch { condition, yes, no } => {
                hash.update([2]);
                word(&mut hash, condition.get() as u64);
                word(&mut hash, yes as u64);
                word(&mut hash, no as u64);
            }
            NativeMatchTerminator::LiteralBranch {
                left,
                right,
                yes,
                no,
            } => {
                hash.update([3]);
                for value in [left.get(), right.get(), yes, no] {
                    word(&mut hash, value as u64);
                }
            }
            NativeMatchTerminator::Complete => hash.update([4]),
            NativeMatchTerminator::Reject => hash.update([5]),
        }
    }
    word(&mut hash, plan.outputs().len() as u64);
    for output in plan.outputs() {
        word(&mut hash, output.output().get() as u64);
        word(&mut hash, output.slot().get() as u64);
        bytes(&mut hash, output.name().as_bytes());
        word(&mut hash, output.value().get() as u64);
    }
    hash.finalize().into()
}
