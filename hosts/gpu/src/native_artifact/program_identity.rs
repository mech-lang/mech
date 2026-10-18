//! Explicit program IR/ABI4 identity. No Debug or pointer serialization.
use mech_engine::native_artifact::{
    NativeIterationAction, NativeIterationTerminator, NativeIterationValueScope, NativeMatchAction,
    NativeMatchTerminator, NativeProgramArtifactPlan, NativeScalarKind,
};
use sha2::{Digest, Sha256};
fn word(hash: &mut Sha256, value: u64) {
    hash.update(value.to_le_bytes());
}
fn bytes(hash: &mut Sha256, value: &[u8]) {
    word(hash, value.len() as u64);
    hash.update(value);
}
pub const NATIVE_PROGRAM_TYPED_IR_VERSION: u64 = 1;
pub const NATIVE_PROGRAM_WORK_POLICY_VERSION: u64 = 1;
pub const NATIVE_PROGRAM_LINK_IDENTITY_BYTES: usize = 192;
const MAGIC: &[u8; 16] = b"MECHNATIVEPROG\0\0";
#[derive(Clone, Debug)]
pub struct NativeProgramObjectIdentity {
    bytes: [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
    symbol: String,
}
impl NativeProgramObjectIdentity {
    pub(super) fn from_descriptor(
        d: &super::program_descriptor::NativeProgramDescriptor,
    ) -> Result<Self, &'static str> {
        let mut bytes = [0; NATIVE_PROGRAM_LINK_IDENTITY_BYTES];
        bytes[..16].copy_from_slice(MAGIC);
        let p = d.plan();
        for (at, value) in [
            (16, NATIVE_PROGRAM_TYPED_IR_VERSION),
            (24, super::program_abi::VERSION),
            (32, NATIVE_PROGRAM_WORK_POLICY_VERSION),
            (112, d.stack_bytes()),
            (120, d.code_bytes()),
            (128, p.inputs().len() as u64),
            (136, p.values().len() as u64),
            (144, p.blocks().len() as u64),
            (152, p.outputs().len() as u64),
            (160, p.entry() as u64),
            (168, p.iterations().len() as u64),
            (176, p.constants().len() as u64),
            (
                184,
                core::mem::size_of::<super::program_abi::Frame>() as u64,
            ),
        ] {
            bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        bytes[40..72].copy_from_slice(p.program_revision().as_bytes());
        bytes[72..104].copy_from_slice(d.typed_ir_sha256());
        bytes[104..112].copy_from_slice(
            &super::identity::host_target_tag().ok_or("unsupported program target")?,
        );
        Ok(Self {
            bytes,
            symbol: format!(
                "mech_canonical_program_identity_v1_{}",
                super::identity::hex(d.typed_ir_sha256())
            ),
        })
    }
    pub fn bytes(&self) -> &[u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES] {
        &self.bytes
    }
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}
pub(super) fn typed_ir_sha256(plan: &NativeProgramArtifactPlan) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"mech.canonical-managed-program-cfg\0");
    word(&mut hash, NATIVE_PROGRAM_TYPED_IR_VERSION); // Separate control IR version.
    word(&mut hash, super::program_abi::VERSION);
    word(&mut hash, NATIVE_PROGRAM_WORK_POLICY_VERSION);
    hash.update(plan.program_revision().as_bytes());
    word(&mut hash, plan.entry() as u64);
    word(&mut hash, plan.values().len() as u64);
    for ty in plan.values() {
        hash.update([match ty.scalar_kind() {
            None => 0,
            Some(NativeScalarKind::Bool) => 1,
            Some(NativeScalarKind::U64) => 2,
            Some(NativeScalarKind::I64) => 3,
            Some(NativeScalarKind::F64) => 4,
        }]);
        word(&mut hash, ty.schema().get() as u64);
        hash.update(ty.schema_key().as_bytes());
    }
    word(&mut hash, plan.inputs().len() as u64);
    for input in plan.inputs() {
        word(&mut hash, input.input().get() as u64);
        word(&mut hash, input.slot().get() as u64);
        bytes(&mut hash, input.name().as_bytes());
        word(&mut hash, input.value().get() as u64);
    }
    word(&mut hash, plan.constants().len() as u64);
    for constant in plan.constants() {
        word(&mut hash, constant.value().get() as u64);
        // The canonical revision commits to complete immutable constant data.
        word(&mut hash, constant.constant().schema().get() as u64);
        hash.update(constant.constant().schema_key().as_bytes());
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
        match block.iteration() {
            Some(id) => {
                hash.update([1]);
                word(&mut hash, id as u64);
            }
            None => hash.update([0]),
        }
        match block.action() {
            // ABI4 commits the maintained selection as well as operand slots.
            NativeIterationAction::LibraryCall {
                template,
                operands,
                output,
            } => {
                hash.update([3]);
                bytes(&mut hash, template.primitive().name().as_bytes());
                word(&mut hash, output.get() as u64);
                word(&mut hash, operands.len() as u64);
                for v in operands {
                    word(&mut hash, v.get() as u64);
                }
                let key = template.selected_key();
                word(&mut hash, key.module_path.len() as u64);
                for part in &key.module_path {
                    bytes(&mut hash, part.as_bytes());
                }
                bytes(&mut hash, key.operation_name.as_bytes());
                use mech_core::ImplementationMemoryClass as M;
                let (tag, a, b) = match template.implementation_memory() {
                    M::NoAdditionalScratch => (0u64, 0, 0),
                    M::CloneInput { input } => (1, input, 0),
                    M::CanonicalCloneInput { input } => (2, input, 0),
                    M::AbiContiguousBridge { input, output } => (3, input, output),
                    M::ExternalMarshalling => (4, 0, 0),
                    M::MatrixSolve => (5, 0, 0),
                    M::CanonicalFinalize => (6, 0, 0),
                    M::CanonicalSortUnique => (7, 0, 0),
                };
                for n in [tag, a as u64, b as u64] {
                    word(&mut hash, n);
                }
            }
            NativeIterationAction::Scalar(action) => {
                hash.update([0]);
                match action {
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
            }
            NativeIterationAction::Initialize { iteration } => {
                hash.update([1]);
                word(&mut hash, *iteration as u64);
            }
            NativeIterationAction::Latch { iteration } => {
                hash.update([2]);
                word(&mut hash, *iteration as u64);
            }
        }
        match block.terminator() {
            NativeIterationTerminator::Scalar(terminator) => {
                hash.update([0]);
                match terminator {
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
            NativeIterationTerminator::Header {
                iteration,
                body,
                exit,
            } => {
                hash.update([1]);
                for id in [iteration, body, exit] {
                    word(&mut hash, id as u64);
                }
            }
        }
    }
    word(&mut hash, plan.value_scopes().len() as u64);
    for scope in plan.value_scopes() {
        match scope {
            NativeIterationValueScope::Immutable { iteration } => {
                hash.update([0]);
                match iteration {
                    Some(id) => {
                        hash.update([1]);
                        word(&mut hash, *id as u64);
                    }
                    None => hash.update([0]),
                }
            }
            NativeIterationValueScope::Carrier { iteration } => {
                hash.update([1]);
                word(&mut hash, *iteration as u64);
            }
        }
    }
    word(&mut hash, plan.iterations().len() as u64);
    for iteration in plan.iterations() {
        word(&mut hash, iteration.origin().node().get() as u64);
        for id in [
            iteration.origin().block(),
            iteration.origin().local(),
            iteration.parent(),
        ] {
            match id {
                Some(id) => {
                    hash.update([1]);
                    word(&mut hash, id as u64);
                }
                None => hash.update([0]),
            }
        }
        word(&mut hash, iteration.count());
        for value in [
            iteration.seed(),
            iteration.carrier(),
            iteration.next(),
            iteration.result(),
        ] {
            word(&mut hash, value.get() as u64);
        }
        for id in [
            iteration.initialize(),
            iteration.header(),
            iteration.body(),
            iteration.latch(),
            iteration.exit(),
        ] {
            word(&mut hash, id as u64);
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
