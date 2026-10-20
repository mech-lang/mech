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
fn selected_template(
    hash: &mut Sha256,
    template: &mech_engine::native_artifact::NativeProgramLibraryTemplate,
) {
    word(hash, template.origin().node().get() as u64);
    word(hash, template.origin().block().map_or(u64::MAX, u64::from));
    word(hash, template.origin().local().map_or(u64::MAX, u64::from));
    hash.update(template.program_revision().as_bytes());
    bytes(hash, template.primitive().name().as_bytes());
    let key = template.selected_key();
    word(hash, key.module_path.len() as u64);
    for part in &key.module_path {
        bytes(hash, part.as_bytes());
    }
    bytes(hash, key.operation_name.as_bytes());
    word(hash, template.inputs().len() as u64);
    for input in template.inputs() {
        bytes(hash, &input.canonical_bytes());
    }
    bytes(hash, &template.output().canonical_bytes());
    match template.contract().canonical_bytes() {
        Ok(encoded) => {
            hash.update([1]);
            bytes(hash, &encoded);
        }
        Err(_) => hash.update([0]), // opaque invalid plans never reach execution
    }
    use mech_core::ImplementationMemoryClass as M;
    let (tag, a, b) = match template.implementation_memory() {
        M::NoAdditionalScratch => (0, 0, 0),
        M::CloneInput { input } => (1, input, 0),
        M::CanonicalCloneInput { input } => (2, input, 0),
        M::AbiContiguousBridge { input, output } => (3, input, output),
        M::ExternalMarshalling => (4, 0, 0),
        M::MatrixSolve => (5, 0, 0),
        M::CanonicalFinalize => (6, 0, 0),
        M::CanonicalSortUnique => (7, 0, 0),
    };
    for number in [tag as u64, a as u64, b as u64] {
        word(hash, number);
    }
}
// IR13 adds bounded dense histogram code to the same closed inclusive-window
// proof. Exclusive selectors retain their distinct ordinary CFG semantics.
pub const NATIVE_PROGRAM_TYPED_IR_VERSION: u64 = 13;
// Shared immutable schema arenas and the admitted two-phase entry protocol
// change work accounting. ABI8/IR5 also commit the exact generated checkpoint;
// neither phase may renew the invocation's cumulative ledger. Policy11 reuses
// exact live-geometry bindings within one invocation, retiring an old binding
// before a shape miss rebuilds it. Every visit measures and executes fresh Values.
// Policy12 replaces repeated managed lexical-scope discovery with exact
// descriptor-derived retirement tables, admitted once on the same session.
// Exact current facts may reuse admitted planning metadata (never values or
// permits). Scalar String capture uses canonical shared-data ownership bounds.
// Immutable String indices are weak-owner-bound, admitted independently, and
// counted in every fresh call/permit; compact boundaries require segmentation.
// Policy13/IR6 add an explicit selected U64 range source strategy. It preserves
// every candidate/error boundary and admits fixed source state instead of a
// nonexistent Matrix. ABI8's existing two-word collection helper wire is unchanged.
// Policy14/IR7 admit selected TextJoin append sinks. The elided Matrix is
// never initialized; its selected consumer alone publishes a canonical String.
// Policy15/IR8 add selected String histogram sinks and guarded query demands.
// No elided Matrix/Map/Set has a Value handle; scalar demand outputs retain
// canonical ownership and every generated producer visit remains unchanged.
// Policy16/IR9 add one private current-candidate unit-U64 String span. Both
// original selected visits remain; the actual source owner and index stay
// fully admitted until the existing sink consumes the checked byte range.
// Policy17 proves immutable selected query-stage associations once in the
// descriptor constructor, eliminating repeated deep equality and unused full
// binder reservations. Every original stage keeps fixed header work, dynamic
// generation/phase checks, comparisons and canonical scalar owner admission.
// ABI8's two-word helper transport and IR9's selected control proof are unchanged.
// Policy18/IR10 add a strict sole-use String element -> selected String map
// entry -> TextJoin strategy. Both original visits and complete owner pins
// remain; only nonexistent key/result allocations and their copies disappear.
// Canonical key planning/search and final append share maintained mechanics.
// Policy20 reuses paid immutable DATA footprints within the same invocation;
// every turn still validates its current finite limits and admission ledger.
// Policy21 composes recursive declared-schema conversion binding and staging
// with the collection strategies above. Present Matrix axes remain witnessed;
// every invocation keeps fresh admission and atomic publication obligations.
// Policy22/IR11 emit closed canonical window-query loops as native regions.
// Their finite byte/compare/instruction bound is charged before entry, not
// by visiting each operation. Existing activation, capture and atomic commit
// remain ONE round. Genuine String owners are loaned only across that body.
// Policy23 prepays a complete native histogram region and finite dense scratch.
pub const NATIVE_PROGRAM_WORK_POLICY_VERSION: u64 = 23;
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
            NativeIterationAction::PrepareStringSpanSelector { collection } => {
                hash.update([14]);
                word(&mut hash, u64::from(*collection));
            }
            NativeIterationAction::ProjectStringSpan { collection } => {
                hash.update([15]);
                word(&mut hash, u64::from(*collection));
            }
            NativeIterationAction::ProjectStringMapEntry { collection } => {
                hash.update([16]);
                word(&mut hash, u64::from(*collection));
            }
            NativeIterationAction::HistogramQueryStep { collection, step } => {
                hash.update([12]);
                word(&mut hash, *collection as u64);
                word(&mut hash, *step as u64);
            }
            NativeIterationAction::HistogramFinish { collection } => {
                hash.update([13]);
                word(&mut hash, *collection as u64);
            }
            NativeIterationAction::TextJoinFinish { collection } => {
                hash.update([11]);
                word(&mut hash, *collection as u64);
            }
            NativeIterationAction::PrepareU64Range { iteration } => {
                hash.update([9]);
                word(&mut hash, u64::from(*iteration));
            }
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
                match collection.source() {
                    mech_engine::native_artifact::NativeCollectionSource::Value(value) => {
                        hash.update([0]);
                        word(&mut hash, u64::from(value.get()));
                    }
                    mech_engine::native_artifact::NativeCollectionSource::U64Range(cursor) => {
                        hash.update([1]);
                        word(&mut hash, u64::from(cursor.prepare()));
                        word(&mut hash, u64::from(cursor.semantic_output().get()));
                        word(&mut hash, u64::from(cursor.before_collection_begin()));
                        for value in cursor.operands() {
                            word(&mut hash, u64::from(value.get()));
                        }
                        let template = cursor.template();
                        hash.update(template.program_revision().as_bytes());
                        bytes(&mut hash, template.primitive().name().as_bytes());
                        let at = template.origin();
                        word(&mut hash, u64::from(at.node().get()));
                        for part in [at.block(), at.local()] {
                            word(&mut hash, part.map_or(u64::MAX, u64::from));
                        }
                        word(&mut hash, template.inputs().len() as u64);
                        for schema in template.inputs() {
                            hash.update(schema.key().as_bytes());
                        }
                        hash.update(template.output().key().as_bytes());
                        let selected = template.selected_key();
                        word(&mut hash, selected.module_path.len() as u64);
                        for part in &selected.module_path {
                            bytes(&mut hash, part.as_bytes());
                        }
                        bytes(&mut hash, selected.operation_name.as_bytes());
                        // The only eligible maintained factories have this
                        // exact class; the engine proof rejects all others.
                        word(
                            &mut hash,
                            u64::from(
                                template.implementation_memory()
                                    == mech_core::ImplementationMemoryClass::NoAdditionalScratch,
                            ),
                        );
                    }
                }
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
        match collection.result() {
            mech_engine::native_artifact::NativeCollectionResult::Materialized => hash.update([0]),
            mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(sink) => {
                use mech_engine::native_artifact::{
                    NativeHistogramQueryPhase as P, NativeHistogramQuerySetShape as S,
                    NativeHistogramQueryStepKind as K,
                };
                hash.update([2]);
                for value in [sink.query(), sink.match_result(), sink.zero()] {
                    word(&mut hash, value.get() as u64);
                }
                hash.update([match sink.set_shape() {
                    S::Dynamic => 0,
                    S::FreeExact => 1,
                }]);
                for site in [sink.true_copy(), sink.false_copy()] {
                    word(&mut hash, site as u64);
                }
                word(&mut hash, sink.bool_tests().len() as u64);
                for &site in sink.bool_tests() {
                    word(&mut hash, site as u64);
                }
                word(&mut hash, sink.steps().len() as u64);
                for step in sink.steps() {
                    word(&mut hash, step.site() as u64);
                    let (tag, phase) = match step.kind() {
                        K::Histogram => (0, 0),
                        K::Keys => (1, 0),
                        K::Set => (2, 0),
                        K::Membership => (3, 0),
                        K::Lookup => (4, 0),
                        K::Cleanup => (5, 0),
                        K::Alias { phase } => (
                            6,
                            match phase {
                                P::ProducerReady => 0,
                                P::HistogramReady => 1,
                                P::KeysReady => 2,
                                P::SetReady => 3,
                                P::Guarded => 4,
                                P::LookupRead => 5,
                            },
                        ),
                    };
                    hash.update([tag, phase]);
                    word(&mut hash, step.operands().len() as u64);
                    for value in step.operands() {
                        word(&mut hash, value.get() as u64);
                    }
                    match step.output() {
                        Some(value) => {
                            hash.update([1]);
                            word(&mut hash, value.get() as u64);
                        }
                        None => hash.update([0]),
                    }
                    match step.template() {
                        Some(template) => {
                            hash.update([1]);
                            selected_template(&mut hash, template);
                        }
                        None => hash.update([0]),
                    }
                }
            }
            mech_engine::native_artifact::NativeCollectionResult::Histogram(sink) => {
                hash.update([3]);
                word(&mut hash, sink.consumer() as u64);
                word(&mut hash, sink.output().get() as u64);
                selected_template(&mut hash, sink.template());
            }
            mech_engine::native_artifact::NativeCollectionResult::TextJoin(sink) => {
                hash.update([1]);
                word(&mut hash, sink.consumer() as u64);
                word(&mut hash, sink.separator().get() as u64);
                word(&mut hash, sink.output().get() as u64);
                let template = sink.template();
                word(&mut hash, template.origin().node().get() as u64);
                word(
                    &mut hash,
                    template.origin().block().map_or(u64::MAX, u64::from),
                );
                word(
                    &mut hash,
                    template.origin().local().map_or(u64::MAX, u64::from),
                );
                hash.update(template.program_revision().as_bytes());
                bytes(&mut hash, template.primitive().name().as_bytes());
                let key = template.selected_key();
                word(&mut hash, key.module_path.len() as u64);
                for part in &key.module_path {
                    bytes(&mut hash, part.as_bytes());
                }
                bytes(&mut hash, key.operation_name.as_bytes());
                word(&mut hash, template.inputs().len() as u64);
                for input in template.inputs() {
                    bytes(&mut hash, &input.canonical_bytes());
                }
                bytes(&mut hash, &template.output().canonical_bytes());
                match template.contract().canonical_bytes() {
                    Ok(encoded) => {
                        hash.update([1]);
                        bytes(&mut hash, &encoded);
                    }
                    Err(_) => hash.update([0]), // invalid private plans are rejected before entry
                }
                hash.update([6]); // exact CanonicalFinalize witness, checked by proof
            }
        }
        match collection.string_span_projection() {
            Some(projection) => {
                hash.update([1]);
                for site in [projection.range_site(), projection.selection_site()] {
                    word(&mut hash, u64::from(site));
                }
                for value in [
                    projection.operands()[0],
                    projection.operands()[1],
                    projection.source(),
                    projection.selector(),
                    projection.output(),
                ] {
                    word(&mut hash, u64::from(value.get()));
                }
                selected_template(&mut hash, projection.range_template());
                selected_template(&mut hash, projection.selection_template());
            }
            None => hash.update([0]),
        }
        match collection.string_element_map_projection() {
            Some(projection) => {
                hash.update([1]);
                for site in [
                    projection.iteration(),
                    projection.element_site(),
                    projection.selection_site(),
                ] {
                    word(&mut hash, u64::from(site));
                }
                for value in [
                    projection.source(),
                    projection.map(),
                    projection.element(),
                    projection.output(),
                ] {
                    word(&mut hash, u64::from(value.get()));
                }
                selected_template(&mut hash, projection.selection_template());
            }
            None => hash.update([0]),
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
