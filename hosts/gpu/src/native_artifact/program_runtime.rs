//! Managed ABI8 invocation. This boundary never evaluates a graph: generated
//! blocks choose every successor and run every scalar/counter instruction.
use super::abi::{
    NO_NODE, NativeEntryFrame, STATUS_ABI, STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK,
};
use super::iteration_abi::STATUS_REJECT;
use super::iteration_runtime::{Context as Control, NativeIterationExecutionError};
use super::memory::{NativeTypedStorage, NativeWordStorage, transport_bytes};
use super::program_abi::{COLLECTION, Frame, LIBRARY, MOVE, OUTPUT_BASE, VERSION};
use super::{
    NativeExecutionError, NativeExecutionLimits, NativeExecutionPhase, NativeProgramDescriptor,
    NativeValueArena, NativeValueError, NativeWorkMeter,
};
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetLimits, Value, ValueData,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDataDraft, ValueDraft},
};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError, NativeIterationAction, NativeIterationTerminator,
    NativeLibraryWorkBudget, NativeMatchAction, NativeMatchOrigin, NativeMatchTerminator,
    NativeMatchValue, NativeScalarKind,
};
use std::ffi::c_void;

#[path = "program_collection.rs"]
mod collection;
#[path = "program_session.rs"]
mod session;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub(super) use session::CheckedEntry;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use session::NativeProgramExecutionSession;

#[derive(Clone, Debug)]
pub enum NativeProgramExecutionError {
    Base(NativeExecutionError),
    Control(NativeIterationExecutionError),
    Value(NativeValueError),
    Library {
        origin: NativeMatchOrigin,
        error: NativeCollectionInvocationError,
    },
    Collection {
        origin: NativeMatchOrigin,
        error: mech_core::ResidentKernelError,
    },
    Snapshot(mech_core::snapshot::SnapshotValueError),
}
impl From<NativeExecutionError> for NativeProgramExecutionError {
    fn from(e: NativeExecutionError) -> Self {
        Self::Base(e)
    }
}
impl From<NativeIterationExecutionError> for NativeProgramExecutionError {
    fn from(e: NativeIterationExecutionError) -> Self {
        Self::Control(e)
    }
}
impl From<NativeValueError> for NativeProgramExecutionError {
    fn from(e: NativeValueError) -> Self {
        Self::Value(e)
    }
}
impl From<mech_core::MemoryRuntimeError> for NativeProgramExecutionError {
    fn from(e: mech_core::MemoryRuntimeError) -> Self {
        Self::Base(e.into())
    }
}
impl std::fmt::Display for NativeProgramExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native program: {self:?}")
    }
}
impl std::error::Error for NativeProgramExecutionError {}
// Preserve the complete typed native failure when crossing the ordinary
// runtime transaction bridge; callers can inspect this exact enum in MechError.
impl mech_core::MechErrorKind for NativeProgramExecutionError {
    fn name(&self) -> &str {
        "NativeProgramExecutionError"
    }
    fn message(&self) -> String {
        self.to_string()
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgramExecutionLimits {
    pub execution: NativeExecutionLimits,
    pub library: MemoryBudgetLimits,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeProgramExecutionReport {
    pub work: u64,
    pub comparison_work: u64,
    pub visits: u64,
    pub transport_bytes: u64,
}
fn invalid(reason: &'static str) -> NativeProgramExecutionError {
    NativeExecutionError::Descriptor { reason }.into()
}
fn amount(count: usize, scale: u64, base: u64) -> Result<u64, NativeProgramExecutionError> {
    (count as u64)
        .checked_mul(scale)
        .and_then(|n| n.checked_add(base))
        .ok_or_else(|| invalid("program size/work overflow"))
}
fn bits(value: &Value, kind: NativeScalarKind) -> Result<u64, NativeProgramExecutionError> {
    match (kind, value.data()) {
        (NativeScalarKind::Bool, ValueData::Bool(v)) => Ok(u64::from(*v)),
        (NativeScalarKind::U64, ValueData::U64(v)) => Ok(*v),
        (NativeScalarKind::I64, ValueData::I64(v)) => Ok(*v as u64),
        (NativeScalarKind::F64, ValueData::F64(v)) => Ok(v.bits()),
        _ => Err(invalid("canonical scalar payload/type mismatch")),
    }
}
struct Context<'plan, 'visit> {
    control: Control<'visit, NativeProgramDescriptor>,
    // Sources are cheap shared roots from the arena; drop collection state
    // first, while that arena still owns their prepaid retirement obligation.
    collections: Option<collection::State>,
    arena: NativeValueArena<'plan>,
    library: NativeLibraryWorkBudget,
    limits: MemoryBudgetLimits,
    result: *mut u64,
    arguments: *mut u64,
    argument_capacity: usize,
    call_inputs: NativeTypedStorage<Option<Value>>,
    consumed: bool,
    output_cursor: usize,
    error: Option<NativeProgramExecutionError>,
    activation_phase: bool,
}
impl Context<'_, '_> {
    fn needs_move(&self, site: usize) -> Option<(NativeMatchValue, NativeMatchValue)> {
        let plan = &self.control.descriptor.plan;
        let pair = match plan.blocks()[site].action() {
            NativeIterationAction::Scalar(NativeMatchAction::Copy { source, output }) => {
                (*source, *output)
            }
            NativeIterationAction::Initialize { iteration } => {
                match plan.iterations()[*iteration as usize].as_ref() {
                    mech_engine::native_artifact::NativeControlLoopRef::Iteration(i) => {
                        (i.seed(), i.carrier())
                    }
                    mech_engine::native_artifact::NativeControlLoopRef::Tail(i) => {
                        (i.seed(), i.carrier())
                    }
                    _ => return None,
                }
            }
            NativeIterationAction::Latch { iteration } => {
                match plan.iterations()[*iteration as usize].as_ref() {
                    mech_engine::native_artifact::NativeControlLoopRef::Iteration(i) => {
                        (i.next(), i.carrier())
                    }
                    mech_engine::native_artifact::NativeControlLoopRef::Tail(i) => {
                        (i.next(), i.carrier())
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };
        plan.values()[pair.1.get() as usize]
            .scalar_kind()
            .is_none()
            .then_some(pair)
    }
    fn action_required(&self, site: usize) -> bool {
        matches!(
            self.control.descriptor.plan.blocks()[site].action(),
            NativeIterationAction::LibraryCall { .. }
                | NativeIterationAction::Structural { .. }
                | NativeIterationAction::CollectionBegin { .. }
                | NativeIterationAction::CollectionElement { .. }
                | NativeIterationAction::CollectionPush { .. }
                | NativeIterationAction::CollectionFinish { .. }
        ) || self.needs_move(site).is_some()
            || matches!(self.control.descriptor.plan.blocks()[site].action(),
                NativeIterationAction::Initialize { iteration }
                if matches!(self.control.descriptor.plan.iterations()[*iteration as usize], mech_engine::native_artifact::NativeControlLoop::Collection(_)))
    }
    fn scalar(
        &mut self,
        value: NativeMatchValue,
        word: u64,
    ) -> Result<Value, NativeProgramExecutionError> {
        let descriptor = self.control.descriptor;
        let ty = &descriptor.plan.values()[value.get() as usize];
        let data = match ty.scalar_kind() {
            Some(NativeScalarKind::Bool) if word <= 1 => ValueDataDraft::Bool(word != 0),
            Some(NativeScalarKind::U64) => ValueDataDraft::U64(word),
            Some(NativeScalarKind::I64) => ValueDataDraft::I64(word as i64),
            Some(NativeScalarKind::F64) => ValueDataDraft::F64(F64Bits::from_bits(word)),
            _ => return Err(invalid("invalid scalar helper operand")),
        };
        // Covers construction, validation, ownership import AND retirement on
        // any later failed insertion. No allocation happens before admission.
        let work = descriptor
            .scalar_bytes
            .checked_mul(8)
            .and_then(|n| n.checked_add(128))
            .ok_or_else(|| invalid("scalar setup work overflow"))?;
        self.control
            .work
            .charge(NativeExecutionPhase::Setup, work)?;
        let memory = self.library.memory_budget();
        let mut reservation = memory.reserve_capacity(descriptor.scalar_bytes)?;
        ValueDraft {
            schema: ty.schema(),
            shape_values: Box::new([]),
            data,
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &descriptor.schemas,
        ))
        .map_err(NativeProgramExecutionError::Snapshot)?
        .into_memory_budget(&mut reservation)
        .map_err(Into::into)
    }
    fn helper(
        &mut self,
        site: u64,
        operation: u64,
        args: [u64; 3],
    ) -> Result<Option<u64>, NativeProgramExecutionError> {
        if self.control.previous.map(u64::from) != Some(site) {
            return Err(invalid("helper outside exact admitted visit"));
        }
        let site = site as usize;
        let descriptor = self.control.descriptor;
        if operation == 0xffff_ffff_ffff_fff0 {
            if self.consumed || args != [0; 3] {
                return Err(invalid("duplicate or malformed structural observation"));
            }
            let NativeIterationAction::Structural {
                source,
                path,
                observation,
                ..
            } = descriptor.plan.blocks()[site].action()
            else {
                return Err(invalid(
                    "structural helper outside its exact canonical site",
                ));
            };
            use mech_engine::native_artifact::{
                NativeStructuralObservation as Observation, NativeStructuralStep as Step,
            };
            let work = (path.len() as u64)
                .checked_mul(32)
                .and_then(|n| n.checked_add(64))
                .ok_or_else(|| invalid("structural observation work overflow"))?;
            self.control.work.charge(
                NativeExecutionPhase::Node(descriptor.plan.blocks()[site].origin().node()),
                work,
            )?;
            let root = self
                .arena
                .copy_current(source.get(), &mut self.control.work)?;
            let mut data = root.data();
            for step in path {
                data = match (step, data) {
                    (Step::EnumPayload { ordinal }, ValueData::Enum(value))
                        if value.ordinal() == *ordinal =>
                    {
                        value
                            .payload()
                            .ok_or_else(|| invalid("structural enum has no payload"))?
                    }
                    (Step::TupleItem { ordinal }, ValueData::Tuple(values)) => values
                        .get(*ordinal as usize)
                        .ok_or_else(|| invalid("structural tuple index"))?,
                    _ => {
                        return Err(invalid(
                            "structural observation path differs from its admitted schema",
                        ));
                    }
                };
            }
            let word = match (observation, data) {
                (Observation::U64, ValueData::U64(value)) => *value,
                (Observation::EnumOrdinal { ordinal }, ValueData::Enum(value)) => {
                    u64::from(value.ordinal() == *ordinal)
                }
                (Observation::StringEqual { peer }, ValueData::String(left)) => {
                    let peer = self
                        .arena
                        .copy_current(peer.get(), &mut self.control.work)?;
                    let ValueData::String(right) = peer.data() else {
                        return Err(invalid("structural String peer has another kind"));
                    };
                    let work = u64::try_from(left.len().max(right.len()))
                        .map_err(|_| invalid("structural String comparison work overflow"))?;
                    // Both finite cumulative compute and comparison accounts
                    // admit the complete worst-case raw-byte equality first.
                    self.library.charge(work, work).map_err(|error| {
                        NativeProgramExecutionError::Collection {
                            origin: descriptor.plan.blocks()[site].origin(),
                            error,
                        }
                    })?;
                    u64::from(left == right)
                }
                _ => {
                    return Err(invalid(
                        "structural result differs from its exact native kind",
                    ));
                }
            };
            self.consumed = true;
            Ok(Some(word))
        } else if operation == LIBRARY {
            if self.consumed {
                return Err(invalid("duplicate library helper"));
            }
            let NativeIterationAction::LibraryCall {
                operands, output, ..
            } = descriptor.plan.blocks()[site].action()
            else {
                return Err(invalid("library helper outside selected call"));
            };
            if args[0] != self.arguments as u64
                || args[1] != operands.len() as u64
                || args[2] != 0
                || operands.len() > self.argument_capacity
            {
                return Err(invalid("library helper operand lane mismatch"));
            }
            // SAFETY: execute owns and initializes this disjoint word lane,
            // registers its exact pointer/capacity above, and retains it for
            // the synchronous entry. Native code is suspended in this callback;
            // no Rust reference to the lane survives across native writes.
            let words = unsafe { core::slice::from_raw_parts(self.arguments, operands.len()) };
            for (index, &operand) in operands.iter().enumerate() {
                let value = if descriptor.plan.values()[operand.get() as usize]
                    .scalar_kind()
                    .is_some()
                {
                    self.scalar(operand, words[index])?
                } else {
                    if words[index] != 0 {
                        return Err(invalid("managed operand has a native payload"));
                    }
                    self.arena
                        .copy_current(operand.get(), &mut self.control.work)?
                };
                self.call_inputs.values_mut()[index] = Some(value);
            }
            let call = descriptor.calls[site]
                .as_ref()
                .ok_or_else(|| invalid("missing exact prepared selection"))?;
            let mut result = None;
            let outcome = call.invoke_current(
                &self.call_inputs.values()[..operands.len()],
                &mut result,
                self.limits,
                &self.library,
            );
            // Every scalar/imported owner prepaid retirement before insertion.
            // Clear both successful and failed calls without retaining an old
            // call's owners while the next site is being constructed.
            self.call_inputs.values_mut()[..operands.len()].fill(None);
            outcome.map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.plan.blocks()[site].origin(),
                error,
            })?;
            let result = result.ok_or_else(|| invalid("library omitted output"))?;
            self.arena
                .put(output.get(), &result, &mut self.control.work)?;
            self.consumed = true;
            descriptor.plan.values()[output.get() as usize]
                .scalar_kind()
                .map(|kind| bits(&result, kind))
                .transpose()
        } else if operation == MOVE {
            if self.consumed {
                return Err(invalid("duplicate managed move"));
            }
            let (source, output) = self
                .needs_move(site)
                .ok_or_else(|| invalid("move outside managed copy/carrier"))?;
            let value = self
                .arena
                .copy_current(source.get(), &mut self.control.work)?;
            self.arena
                .put(output.get(), &value, &mut self.control.work)?;
            if let NativeIterationAction::Latch { iteration } =
                descriptor.plan.blocks()[site].action()
                && let mech_engine::native_artifact::NativeControlLoop::Tail(tail) =
                    &descriptor.plan.iterations()[*iteration as usize]
            {
                self.retire_tail_locals(*iteration, tail.carrier())?;
            }
            self.consumed = true;
            Ok(None)
        } else if operation == COLLECTION {
            if self.consumed {
                return Err(invalid("duplicate collection helper"));
            }
            let mut state = self
                .collections
                .take()
                .ok_or_else(|| invalid("collection helper without state"))?;
            let result = state.helper(self, site, args[0]);
            self.collections = Some(state);
            let result = result?;
            self.consumed = true;
            Ok(result)
        } else {
            if operation != OUTPUT_BASE + self.output_cursor as u64
                || (self.action_required(site) && !self.consumed)
                || descriptor.plan.blocks()[site].terminator()
                    != NativeIterationTerminator::Scalar(NativeMatchTerminator::Complete)
            {
                return Err(invalid("typed output outside exact completion order"));
            }
            if self.output_cursor >= descriptor.plan.transport_output_count() {
                return Err(invalid("extra typed output"));
            }
            let out = descriptor.plan.transport_output_value(self.output_cursor);
            let ty = descriptor.plan.transport_output_type(self.output_cursor);
            let value = if ty.scalar_kind().is_some() {
                self.scalar(out, args[0])?
            } else {
                self.arena.copy_current(out.get(), &mut self.control.work)?
            };
            let target = descriptor
                .plan
                .values()
                .len()
                .checked_add(self.output_cursor)
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| invalid("typed output slot overflow"))?;
            self.arena.put(target, &value, &mut self.control.work)?;
            self.output_cursor += 1;
            Ok(None)
        }
    }
    fn visit(&mut self, site: u64) -> Result<(), NativeProgramExecutionError> {
        if self.activation_phase
            && self
                .control
                .descriptor
                .plan
                .activation_boundary()
                .map(u64::from)
                == Some(site)
        {
            return Err(invalid(
                "activation checkpoint must return before its fresh visit",
            ));
        }
        if let Some(previous) = self.control.previous
            && self.action_required(previous as usize)
            && !self.consumed
        {
            return Err(invalid("entry omitted current required helper"));
        }
        self.control.visit(site)?;
        let descriptor = self.control.descriptor;
        if let super::iteration_abi::Progress::Exit(iteration) =
            descriptor.visits[site as usize].progress
            && let mech_engine::native_artifact::NativeControlLoop::Tail(tail) =
                &descriptor.plan.iterations()[iteration as usize]
        {
            // Keep only the final return until the following generated copy.
            self.retire_tail_locals(iteration, tail.returned())?;
        }
        if let super::iteration_abi::Progress::Exit(iteration) =
            descriptor.visits[site as usize].progress
            && matches!(
                descriptor.plan.iterations()[iteration as usize],
                mech_engine::native_artifact::NativeControlLoop::Collection(_)
            )
        {
            let mut state = self
                .collections
                .take()
                .ok_or_else(|| invalid("collection exit without state"))?;
            let result = state.release_source(self, iteration);
            self.collections = Some(state);
            result?;
        }
        if self.control.completed
            && self
                .collections
                .as_ref()
                .is_some_and(|state| !state.is_complete())
        {
            return Err(invalid("completion with an active collection builder"));
        }
        self.consumed = false;
        Ok(())
    }

    fn retire_tail_locals(
        &mut self,
        iteration: u32,
        keep: NativeMatchValue,
    ) -> Result<(), NativeProgramExecutionError> {
        let plan = &self.control.descriptor.plan;
        let scan = (plan.values().len() as u64)
            .checked_mul(
                (plan.iterations().len() as u64)
                    .checked_add(1)
                    .and_then(|n| n.checked_mul(16))
                    .ok_or_else(|| invalid("tail scope work overflow"))?,
            )
            .and_then(|n| n.checked_add(64))
            .ok_or_else(|| invalid("tail scope work overflow"))?;
        self.control
            .work
            .charge(NativeExecutionPhase::Setup, scan)?;
        for (slot, scope) in plan.value_scopes().iter().enumerate() {
            if slot == keep.get() as usize || plan.values()[slot].scalar_kind().is_some() {
                continue;
            }
            let mut owner = match scope {
                mech_engine::native_artifact::NativeIterationValueScope::Immutable {
                    iteration,
                } => *iteration,
                mech_engine::native_artifact::NativeIterationValueScope::Carrier { iteration } => {
                    Some(*iteration)
                }
            };
            while let Some(id) = owner {
                if id == iteration {
                    self.arena
                        .retire_current(slot as u32, &mut self.control.work)?;
                    break;
                }
                owner = plan.iterations()[id as usize].as_ref().parent();
            }
        }
        Ok(())
    }
}
unsafe extern "C" fn admit(context: *mut c_void, site: u64) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: execute retains this invocation context and disjoint backing for
    // the synchronous private entry; no pointer escapes the call.
    let context = unsafe { &mut *context.cast::<Context<'_, '_>>() };
    if context.error.is_some() {
        return STATUS_ADMISSION;
    }
    match context.visit(site) {
        Ok(()) => STATUS_OK,
        Err(error) => {
            context.error = Some(error);
            STATUS_ADMISSION
        }
    }
}
unsafe extern "C" fn helper(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: the private entry receives only execute's retained context.
    let context = unsafe { &mut *context.cast::<Context<'_, '_>>() };
    if context.error.is_some() {
        return STATUS_ADMISSION;
    }
    let outcome = if result.is_null() || result != context.result {
        Err(invalid("helper result pointer mismatch"))
    } else {
        context.helper(site, operation, [first, second, third])
    };
    match outcome {
        Ok(word) => {
            if let Some(word) = word {
                // SAFETY: equality above proves this is the private one-word
                // result lane, disjoint from borrowed control and Value arenas.
                unsafe {
                    *result = word;
                }
            }
            STATUS_OK
        }
        Err(error) => {
            context.error = Some(error);
            STATUS_ADMISSION
        }
    }
}
pub(super) fn execute<F: FnMut(*mut Frame) -> u64>(
    descriptor: &NativeProgramDescriptor,
    entry: F,
    inputs: &[Value],
    outputs: &mut [Option<Value>],
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    // A raw immutable Value slice is not resource authority, even when every
    // resource result could be supplied as a lookalike typed input.
    if !descriptor.plan.resource_requirements().is_empty() {
        return Err(invalid(
            "hosted native plan requires a runtime-issued turn token",
        ));
    }
    execute_prepared(descriptor, entry, inputs, &[], outputs, limits, memory)
}

#[cfg(feature = "canonical-native-program-host-runtime")]
fn verify_hosted_plan(
    descriptor: &NativeProgramDescriptor,
    revision: mech_core::ProgramRevision,
    bound: &mech_runtime::CanonicalNativeResourcePlan,
) -> Result<(), NativeProgramExecutionError> {
    use mech_runtime::CanonicalNativeResourceAction as Action;
    let selected = &descriptor.plan;
    if revision != selected.program_revision()
        || bound.requirements() != selected.resource_requirements()
        || bound.observations().len() != selected.observations().len()
        || bound.effects().len() != selected.effects().len()
    {
        return Err(invalid(
            "runtime turn differs from canonical native resource plan",
        ));
    }
    for (bound, native) in bound.observations().iter().zip(selected.observations()) {
        if bound.node() != native.node()
            || bound.requirement() != native.requirement()
            || bound.request() != native.request()
            || bound.schema() != native.ty().schema()
            || bound.schema_key() != native.ty().schema_key()
            || bound.action()
                != (Action::Observation {
                    output: native.slot(),
                })
        {
            return Err(invalid(
                "runtime observation differs from exact native site",
            ));
        }
    }
    for (bound, native) in bound.effects().iter().zip(selected.effects()) {
        if bound.node() != native.node()
            || bound.requirement() != native.requirement()
            || bound.request() != native.request()
            || bound.schema() != native.ty().schema()
            || bound.schema_key() != native.ty().schema_key()
            || bound.action()
                != (Action::Effect {
                    payload: native.payload(),
                    ordinal: native.ordinal(),
                })
        {
            return Err(invalid("runtime effect differs from exact native site"));
        }
    }
    Ok(())
}

#[cfg(feature = "canonical-native-program-host-runtime")]
fn verify_hosted_turn(
    descriptor: &NativeProgramDescriptor,
    turn: &mech_runtime::CanonicalNativeResourceTurn<'_>,
) -> Result<(), NativeProgramExecutionError> {
    verify_hosted_plan(descriptor, turn.program_revision(), turn.plan())?;
    if turn.observations().len() != descriptor.plan.observations().len() {
        return Err(invalid(
            "captured observation count differs from exact native plan",
        ));
    }
    Ok(())
}

#[cfg(feature = "canonical-native-program-host-runtime")]
pub(super) fn prepare_hosted<'a>(
    descriptor: &'a NativeProgramDescriptor,
    entry: CheckedEntry,
    admission: mech_runtime::CanonicalNativeResourceAdmission<'_>,
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionSession<'a>, NativeProgramExecutionError> {
    NativeProgramExecutionSession::prepare(descriptor, entry, admission, limits, memory)
}

// Ordinary pure evaluation drives the SAME generated phases internally.
// Fresh input/output transport begins only after closed activation succeeds.
fn execute_prepared<F: FnMut(*mut Frame) -> u64>(
    descriptor: &NativeProgramDescriptor,
    mut entry: F,
    inputs: &[Value],
    observations: &[Value],
    outputs: &mut [Option<Value>],
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    let mut prepared = session::Prepared::new(descriptor, limits, memory)?;
    prepared.activate(&mut entry)?;
    prepared.finish(&mut entry, inputs, observations, outputs)
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests {
    use super::*;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    fn artifact() -> mech_engine::ProgramArtifact {
        artifact_source("+> sequence\nsequence/length(seed<string>)\n")
    }
    fn artifact_source(source: &str) -> mech_engine::ProgramArtifact {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x50524f54), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        mech_engine::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap()
    }
    fn limits() -> NativeProgramExecutionLimits {
        let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        library.max_compute_work = Some(134_217_728);
        library.max_temporary_bytes = Some(128 * 1024 * 1024);
        NativeProgramExecutionLimits {
            execution: NativeExecutionLimits {
                max_work: 1_000_000_000,
                ..NativeExecutionLimits::default()
            },
            library,
        }
    }
    fn input(artifact: &mech_engine::ProgramArtifact, memory: &ManagedMemoryBudget) -> Value {
        let value = ValueDraft {
            schema: artifact.inputs()[0].schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::String("abc".into()),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let mut reservation = memory
            .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
            .unwrap();
        value.into_memory_budget(&mut reservation).unwrap()
    }
    // Protocol-forgery fixtures below have no closed operations. Simulate only
    // their empty entry/checkpoint before injecting a malformed TURN sequence.
    fn checkpoint_empty_activation(
        frame: &mut Frame,
        descriptor: &NativeProgramDescriptor,
    ) -> bool {
        if frame.phase != super::super::program_abi::PHASE_ACTIVATION {
            return false;
        }
        assert!(!descriptor.plan.has_activation());
        assert_eq!(
            unsafe { (frame.base.admit_next)(frame.base.context, descriptor.plan.entry() as u64) },
            STATUS_OK
        );
        frame.base.error_node = u64::from(descriptor.plan.activation_boundary().unwrap());
        true
    }
    fn reach_call(frame: &mut Frame, descriptor: &NativeProgramDescriptor) -> u32 {
        let mut site = descriptor.plan.activation_boundary().unwrap();
        loop {
            // SAFETY: injected tests use execute's retained callback and frame.
            assert_eq!(
                unsafe { (frame.base.admit_next)(frame.base.context, site as u64) },
                STATUS_OK
            );
            let block = &descriptor.plan.blocks()[site as usize];
            if matches!(block.action(), NativeIterationAction::LibraryCall { .. }) {
                return site;
            }
            let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
                block.terminator()
            else {
                panic!("expected straight-line fixture")
            };
            site = next;
        }
    }
    #[test]
    fn helper_protocol_denies_wrong_site_kind_pointer_duplicate_and_omission_atomically() {
        let artifact = artifact();
        let jit =
            super::super::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        for mode in 0..9 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = [input(&artifact, &memory)];
            let retained = memory.used_bytes();
            let mut outputs = vec![None; artifact.outputs().len()];
            let error = execute(
                jit.descriptor(),
                |raw| {
                    // SAFETY: execute owns this initialized live frame throughout
                    // the synchronous injected test entry; backing stays disjoint.
                    let frame = unsafe { &mut *raw };
                    if checkpoint_empty_activation(frame, jit.descriptor()) {
                        return super::super::program_abi::STATUS_ACTIVATED;
                    }
                    let site = reach_call(frame, jit.descriptor());
                    let (actual_site, operation, result) = match mode {
                        0 => (site as u64 + 1, LIBRARY, frame.result),
                        1 => (site as u64, MOVE, frame.result),
                        2 => (site as u64, LIBRARY, frame.result.wrapping_add(1)),
                        _ => (site as u64, LIBRARY, frame.result),
                    };
                    if mode == 4 {
                        let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
                            jit.descriptor().plan.blocks()[site as usize].terminator()
                        else {
                            panic!("fixture terminator")
                        };
                        // SAFETY: same retained callback/context, deliberately
                        // skipping the required leaf to test protocol denial.
                        return unsafe { (frame.base.admit_next)(frame.base.context, next as u64) };
                    }
                    let first = if mode == 5 {
                        frame.arguments.wrapping_add(1) as u64
                    } else {
                        frame.arguments as u64
                    };
                    let arity = match jit.descriptor().plan.blocks()[site as usize].action() {
                        NativeIterationAction::LibraryCall { operands, .. } => {
                            operands.len() as u64
                        }
                        _ => unreachable!(),
                    };
                    let second = if mode == 6 { arity + 1 } else { arity };
                    let third = u64::from(mode == 7);
                    if mode == 8 {
                        // SAFETY: this is the owned, initialized first operand
                        // word of text/lines; managed operands must be zero.
                        unsafe {
                            *frame.arguments = 1;
                        }
                    }
                    // SAFETY: arguments remain within the retained ABI8 boundary;
                    // wrong metadata/pointer is rejected before dereference.
                    let status = unsafe {
                        (frame.helper)(
                            frame.base.context,
                            actual_site,
                            operation,
                            first,
                            second,
                            third,
                            result,
                        )
                    };
                    if mode == 3 {
                        assert_eq!(status, STATUS_OK);
                        // SAFETY: same live context/result lane, deliberate repeat.
                        unsafe {
                            (frame.helper)(
                                frame.base.context,
                                actual_site,
                                operation,
                                first,
                                second,
                                0,
                                result,
                            )
                        }
                    } else {
                        status
                    }
                },
                &args,
                &mut outputs,
                limits(),
                &memory,
            )
            .unwrap_err();
            assert!(
                matches!(
                    error,
                    NativeProgramExecutionError::Base(NativeExecutionError::Descriptor { .. })
                ),
                "mode {mode}: {error:?}"
            );
            assert!(outputs.iter().all(Option::is_none));
            assert_eq!(memory.used_bytes(), retained);
        }
    }
    #[test]
    fn phase_protocol_rejects_forged_early_checkpoint_completion_and_frame_phase() {
        let artifact = artifact();
        let jit =
            super::super::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        for mode in 0..4 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = [input(&artifact, &memory)];
            let before = memory.used_bytes();
            let mut output = vec![None; artifact.outputs().len()];
            let result = execute(
                jit.descriptor(),
                |raw| {
                    // SAFETY: the test receives this synchronous owned frame only.
                    let frame = unsafe { &mut *raw };
                    let boundary = jit.descriptor().plan.activation_boundary().unwrap();
                    if mode == 0 {
                        frame.phase = super::super::program_abi::PHASE_TURN;
                    } else if mode == 1 {
                        return STATUS_OK;
                    } else if mode == 2 {
                        // No admitted activation entry, despite a forged origin.
                        frame.base.error_node = u64::from(boundary);
                    } else {
                        return unsafe {
                            (frame.base.admit_next)(frame.base.context, u64::from(boundary))
                        };
                    }
                    super::super::program_abi::STATUS_ACTIVATED
                },
                &args,
                &mut output,
                limits(),
                &memory,
            );
            assert!(result.is_err(), "forged phase mode {mode}");
            assert!(output.iter().all(Option::is_none));
            assert_eq!(memory.used_bytes(), before);
        }
    }

    #[test]
    fn setup_denies_before_entry_and_before_foreign_payload_type_scan() {
        let artifact = artifact();
        let jit =
            super::super::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        let foreign = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = [input(&artifact, &foreign)];
        for zero_work in [false, true] {
            let memory = ManagedMemoryBudget::new(0);
            let called = std::cell::Cell::new(false);
            let mut outputs = vec![None; artifact.outputs().len()];
            let mut policy = limits();
            if zero_work {
                policy.execution.max_work = 0;
            }
            let error = execute(
                jit.descriptor(),
                |_| {
                    called.set(true);
                    STATUS_ABI
                },
                &args,
                &mut outputs,
                policy,
                &memory,
            )
            .unwrap_err();
            assert!(!called.get());
            if zero_work {
                assert!(matches!(
                    error,
                    NativeProgramExecutionError::Base(NativeExecutionError::WorkLimit { .. })
                ));
            } else {
                assert!(matches!(
                    error,
                    NativeProgramExecutionError::Base(NativeExecutionError::Memory { .. })
                ));
            }
            assert!(outputs.iter().all(Option::is_none));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    #[test]
    fn independent_program_entry_cannot_refill_an_active_library_account() {
        let artifact = artifact();
        let jit =
            super::super::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = [input(&artifact, &memory)];
        let mut work = NativeWorkMeter::new(1000).unwrap();
        let library = work
            .shared_library_budget(NativeExecutionPhase::Setup, 100, &memory)
            .unwrap();
        let before = (library.used(), memory.used_bytes());
        library.with_scope(|| {
            let mut outputs=vec![None;artifact.outputs().len()];
            assert!(matches!(jit.evaluate_into(&args,&mut outputs,limits(),&memory),Err(NativeProgramExecutionError::Base(NativeExecutionError::Descriptor{reason:"independent native work ledger cannot replace an active invocation"}))));
            assert!(outputs.iter().all(Option::is_none));
        }).unwrap();
        assert_eq!((library.used(), memory.used_bytes()), before);
    }
    #[test]
    fn collection_protocol_denies_forged_omitted_repeated_and_early_visits_atomically() {
        let artifact = artifact_source("[letter | letter <- seed<string>]\n");
        let jit =
            super::super::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        let descriptor = jit.descriptor();
        let collection = &descriptor.plan.collections()[0];
        let mech_engine::native_artifact::NativeControlLoop::Collection(generator) =
            &descriptor.plan.iterations()[collection.generators()[0] as usize]
        else {
            panic!("collection generator")
        };
        for mode in 0..9 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = [input(&artifact, &memory)];
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            let prior = outputs[0]
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap();
            let retained = memory.used_bytes();
            let error = execute(
                descriptor,
                |raw| {
                    // SAFETY: execute retains this initialized exclusive frame,
                    // callback context and disjoint result lane until return.
                    let frame = unsafe { &mut *raw };
                    if checkpoint_empty_activation(frame, descriptor) {
                        return super::super::program_abi::STATUS_ACTIVATED;
                    }
                    let admit = |site| {
                        // SAFETY: the same live private callback/context; only
                        // visit metadata is varied, never pointer ownership.
                        unsafe { (frame.base.admit_next)(frame.base.context, site as u64) }
                    };
                    let invoke = |site, operation, result| {
                        // SAFETY: retained private helper/context; a deliberately
                        // wrong result pointer is checked before dereference.
                        unsafe {
                            (frame.helper)(
                                frame.base.context,
                                site as u64,
                                operation,
                                0,
                                0,
                                0,
                                result,
                            )
                        }
                    };
                    let next = |site: u32| {
                        let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
                            descriptor.plan.blocks()[site as usize].terminator()
                        else {
                            panic!("straight-line collection setup")
                        };
                        next
                    };
                    let mut site = descriptor.plan.activation_boundary().unwrap();
                    loop {
                        assert_eq!(admit(site), STATUS_OK);
                        if site == collection.begin() {
                            break;
                        }
                        site = next(site);
                    }
                    match mode {
                        0 => return invoke(site + 1, COLLECTION, frame.result),
                        1 => return invoke(site, MOVE, frame.result),
                        2 => return invoke(site, COLLECTION, frame.result.wrapping_add(1)),
                        4 => return admit(next(site)),
                        _ => {}
                    }
                    assert_eq!(invoke(site, COLLECTION, frame.result), STATUS_OK);
                    if mode == 3 {
                        return invoke(site, COLLECTION, frame.result);
                    }
                    loop {
                        site = next(site);
                        assert_eq!(admit(site), STATUS_OK);
                        if site == generator.initialize() {
                            break;
                        }
                    }
                    if mode == 5 {
                        return admit(generator.header());
                    }
                    assert_eq!(invoke(site, COLLECTION, frame.result), STATUS_OK);
                    if mode == 6 {
                        return invoke(site, COLLECTION, frame.result);
                    }
                    assert_eq!(admit(generator.header()), STATUS_OK);
                    if mode == 7 {
                        return admit(generator.exit());
                    }
                    assert_eq!(admit(generator.body()), STATUS_OK);
                    admit(next(generator.body()))
                },
                &args,
                &mut outputs,
                limits(),
                &memory,
            )
            .unwrap_err();
            if mode == 7 {
                assert!(
                    matches!(error, NativeProgramExecutionError::Control(
                        NativeIterationExecutionError::Visit { previous: Some(previous), actual }
                    ) if previous == generator.header() && actual == u64::from(generator.exit())),
                    "mode {mode}: {error:?}"
                );
            } else {
                assert!(
                    matches!(
                        error,
                        NativeProgramExecutionError::Base(NativeExecutionError::Descriptor { .. })
                    ),
                    "mode {mode}: {error:?}"
                );
            }
            assert_eq!(memory.used_bytes(), retained, "mode {mode}");
            assert_eq!(
                outputs[0]
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap(),
                prior,
                "mode {mode}"
            );
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            assert_eq!(
                outputs[0]
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap(),
                prior
            );
        }
    }
    #[test]
    fn managed_link_metadata_rederives_semantics_and_rejects_mutations_without_an_entry() {
        let source = "+> text\n+> sequence\n+> number\nwords := text/lines(seed<string>)\ntext/join([number/format(sequence/length(word), 0u64) | word <- words], \",\")\n";
        let original = artifact_source(source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        let different = artifact_source(&source.replace("0u64", "1u64"));
        let catalog = mech_stdlib::source_catalog();
        let empty = mech_core::FunctionCatalogBuilder::new().build().unwrap();
        let prepare = super::super::program_linked::prepare_descriptor;
        for artifact in [original, decoded] {
            let object = super::super::emit_program_object(&artifact, &catalog).unwrap();
            let identity = *object.identity().bytes();
            let descriptor = prepare(&artifact, &catalog, &identity).unwrap();
            assert_eq!(
                descriptor.typed_ir_sha256(),
                object.descriptor().typed_ir_sha256()
            );
            for offset in 0..identity.len() {
                // These two nonzero resource claims require build-owned Object
                // provenance, tested by the real linked pin-mutation qualifier.
                // They are not falsely treated as runtime-rederived semantics.
                if (112..128).contains(&offset) {
                    continue;
                }
                let mut mutated = identity;
                mutated[offset] ^= 1;
                assert!(
                    prepare(&artifact, &catalog, &mutated).is_err(),
                    "identity byte {offset}"
                );
            }
            for (offset, old) in [
                (16, 1u64),
                (24, 4u64),
                (24, 5u64),
                (24, 6u64),
                (24, 7u64),
                (16, 4u64),
                (32, 7u64),
                (32, 8u64),
                (32, 1u64),
                (32, 2u64),
                (32, 3u64),
                (32, 4u64),
                (112, 0u64),
                (120, 0u64),
            ] {
                let mut mutated = identity;
                mutated[offset..offset + 8].copy_from_slice(&old.to_le_bytes());
                assert!(
                    prepare(&artifact, &catalog, &mutated).is_err(),
                    "identity field {offset}"
                );
            }
            assert!(prepare(&different, &catalog, &identity).is_err());
            assert!(prepare(&artifact, &empty, &identity).is_err());
        }
    }
    #[test]
    fn abi8_keeps_the_frozen_scalar_prefix_and_disjoint_helper_lanes() {
        assert_eq!(VERSION, 8);
        assert_eq!(core::mem::offset_of!(Frame, base), 0);
        assert_eq!(
            core::mem::offset_of!(Frame, helper),
            core::mem::size_of::<NativeEntryFrame>()
        );
        assert_eq!(
            core::mem::offset_of!(Frame, result),
            core::mem::size_of::<NativeEntryFrame>() + 8
        );
        assert_eq!(
            core::mem::offset_of!(Frame, arguments),
            core::mem::size_of::<NativeEntryFrame>() + 16
        );
        assert_eq!(
            core::mem::offset_of!(Frame, argument_capacity),
            core::mem::size_of::<NativeEntryFrame>() + 24
        );
        assert_eq!(
            core::mem::offset_of!(Frame, phase),
            core::mem::size_of::<NativeEntryFrame>() + 32
        );
        assert_eq!(
            core::mem::size_of::<Frame>(),
            core::mem::size_of::<NativeEntryFrame>() + 40
        );
    }
}
