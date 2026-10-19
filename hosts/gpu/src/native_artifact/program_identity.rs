//! Explicit program IR/ABI8 identity. No Debug or pointer serialization.
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
pub const NATIVE_PROGRAM_TYPED_IR_VERSION: u64 = 5;
// Shared immutable schema arenas and the admitted two-phase entry protocol
// change work accounting. ABI8/IR5 also commit the exact generated checkpoint;
// neither phase may renew the invocation's cumulative ledger. Policy11 reuses
// exact live-geometry bindings within one invocation, retiring an old binding
// before a shape miss rebuilds it. Every visit measures and executes fresh Values.
pub const NATIVE_PROGRAM_WORK_POLICY_VERSION: u64 = 11;
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
            (128, p.transport_input_count() as u64),
            (136, p.values().len() as u64),
            (144, p.blocks().len() as u64),
            (152, p.transport_output_count() as u64),
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
    word(
        &mut hash,
        plan.activation_boundary().map_or(u64::MAX, u64::from),
    );
    word(&mut hash, plan.activation_nodes().len() as u64);
    for node in plan.activation_nodes() {
        word(&mut hash, u64::from(node.get()));
    }
    word(&mut hash, plan.inputs().len() as u64);
    for input in plan.inputs() {
        word(&mut hash, input.input().get() as u64);
        word(&mut hash, input.slot().get() as u64);
        bytes(&mut hash, input.name().as_bytes());
        word(&mut hash, input.value().get() as u64);
    }
    // Retain complete original resource authority obligations, including unused
    // declarations. Observation/effect SSA transport is explicit compiler
    // metadata, not a synthetic application input/output or a resource opcode.
    word(&mut hash, plan.resource_requirements().len() as u64);
    for request in plan.resource_requirements() {
        for field in [
            &request.base_uri,
            &request.path,
            &request.context_name,
            &request.operation,
        ] {
            bytes(&mut hash, field.as_bytes());
        }
        hash.update([match request.intent {
            mech_core::ResourceIntent::Read => 0,
            mech_core::ResourceIntent::Send => 1,
            mech_core::ResourceIntent::Assign => 2,
        }]);
        hash.update([match request.delivery {
            mech_core::ResourceDelivery::Snapshot => 0,
            mech_core::ResourceDelivery::Live => 1,
        }]);
    }
    word(&mut hash, plan.observations().len() as u64);
    for observation in plan.observations() {
        for field in [
            observation.node().get(),
            observation.requirement().get(),
            observation.slot().get(),
            observation.ordinal(),
            observation.value().get(),
            observation.ty().schema().get(),
        ] {
            word(&mut hash, u64::from(field));
        }
        hash.update(observation.ty().schema_key().as_bytes());
    }
    word(&mut hash, plan.effects().len() as u64);
    for effect in plan.effects() {
        for field in [
            effect.node().get(),
            effect.requirement().get(),
            effect.ordinal(),
            effect.value().get(),
            effect.ty().schema().get(),
        ] {
            word(&mut hash, u64::from(field));
        }
        hash.update(effect.ty().schema_key().as_bytes());
        match effect.payload() {
            mech_engine::ArtifactSource::Slot(slot) => {
                hash.update([0]);
                word(&mut hash, u64::from(slot.get()));
            }
            mech_engine::ArtifactSource::Constant(constant) => {
                hash.update([1]);
                word(&mut hash, u64::from(constant.get()));
            }
        }
    }
    word(&mut hash, plan.transport_input_count() as u64);
    word(&mut hash, plan.transport_output_count() as u64);
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
            NativeIterationAction::Structural {
                source,
                path,
                observation,
                output,
            } => {
                hash.update([8]);
                word(&mut hash, source.get() as u64);
                word(&mut hash, output.get() as u64);
                word(&mut hash, path.len() as u64);
                for step in path {
                    match step {
                        mech_engine::native_artifact::NativeStructuralStep::EnumPayload {
                            ordinal,
                        } => {
                            hash.update([0]);
                            word(&mut hash, *ordinal as u64);
                        }
                        mech_engine::native_artifact::NativeStructuralStep::TupleItem {
                            ordinal,
                        } => {
                            hash.update([1]);
                            word(&mut hash, *ordinal as u64);
                        }
                    }
                }
                match observation {
                    mech_engine::native_artifact::NativeStructuralObservation::U64 => {
                        hash.update([0])
                    }
                    mech_engine::native_artifact::NativeStructuralObservation::EnumOrdinal {
                        ordinal,
                    } => {
                        hash.update([1]);
                        word(&mut hash, *ordinal as u64);
                    }
                    mech_engine::native_artifact::NativeStructuralObservation::StringEqual {
                        peer,
                    } => {
                        hash.update([2]);
                        word(&mut hash, peer.get() as u64);
                    }
                }
            }
            // Commit the maintained selection as well as operand slots.
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
            NativeIterationAction::CollectionBegin { collection } => {
                hash.update([4]);
                word(&mut hash, *collection as u64);
            }
            NativeIterationAction::CollectionElement { iteration } => {
                hash.update([5]);
                word(&mut hash, *iteration as u64);
            }
            NativeIterationAction::CollectionPush { collection, value } => {
                hash.update([6]);
                word(&mut hash, *collection as u64);
                word(&mut hash, value.get() as u64);
            }
            NativeIterationAction::CollectionFinish { collection } => {
                hash.update([7]);
                word(&mut hash, *collection as u64);
            }
        }
        match block.terminator() {
            NativeIterationTerminator::TailHeader { iteration, body } => {
                hash.update([2]);
                word(&mut hash, iteration as u64);
                word(&mut hash, body as u64);
            }
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
        let iteration = iteration.as_ref();
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
        match iteration {
            mech_engine::native_artifact::NativeControlLoopRef::Iteration(fold) => {
                hash.update([0]);
                word(&mut hash, fold.count());
                for value in [fold.seed(), fold.carrier(), fold.next(), fold.result()] {
                    word(&mut hash, value.get() as u64);
                }
            }
            mech_engine::native_artifact::NativeControlLoopRef::Collection(collection) => {
                hash.update([1]);
                word(&mut hash, collection.collection() as u64);
                word(&mut hash, collection.source().get() as u64);
                match collection.element() {
                    Some(value) => {
                        hash.update([1]);
                        word(&mut hash, value.get() as u64);
                    }
                    None => hash.update([0]),
                }
            }
            mech_engine::native_artifact::NativeControlLoopRef::Tail(tail) => {
                hash.update([2]);
                for value in [
                    tail.seed(),
                    tail.carrier(),
                    tail.next(),
                    tail.returned(),
                    tail.result(),
                ] {
                    word(&mut hash, value.get() as u64);
                }
                word(&mut hash, tail.finish() as u64);
            }
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

    word(&mut hash, plan.collections().len() as u64);
    for collection in plan.collections() {
        word(&mut hash, collection.origin().node().get() as u64);
        for id in [
            collection.origin().block(),
            collection.origin().local(),
            collection.parent(),
            collection.parent_collection(),
        ] {
            match id {
                Some(id) => {
                    hash.update([1]);
                    word(&mut hash, id as u64);
                }
                None => hash.update([0]),
            }
        }
        match collection.kind() {
            mech_engine::ComprehensionKind::Matrix => hash.update([0]),
            mech_engine::ComprehensionKind::MatrixPreserveShape => hash.update([1]),
            mech_engine::ComprehensionKind::Set => hash.update([2]),
            mech_engine::ComprehensionKind::MatrixHorizontalBlocks { rows } => {
                hash.update([3]);
                match rows {
                    mech_engine::MatrixBlockRows::InferOrZero => hash.update([0]),
                    mech_engine::MatrixBlockRows::Fixed(n) => {
                        hash.update([1]);
                        word(&mut hash, n);
                    }
                    mech_engine::MatrixBlockRows::InputMatrixElements { input } => {
                        hash.update([2]);
                        word(&mut hash, input as u64);
                    }
                }
            }
        }
        for value in [
            collection.output().get(),
            collection.begin(),
            collection.push(),
            collection.finish(),
        ] {
            word(&mut hash, value as u64);
        }
        word(&mut hash, collection.generators().len() as u64);
        for &generator in collection.generators() {
            word(&mut hash, generator as u64);
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
