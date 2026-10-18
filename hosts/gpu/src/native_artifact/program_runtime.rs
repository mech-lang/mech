//! Managed ABI5 invocation. This boundary never evaluates a graph: generated
//! blocks choose every successor and run every scalar/counter instruction.
use super::abi::{
    NO_NODE, NativeEntryFrame, STATUS_ABI, STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK,
};
use super::iteration_abi::STATUS_REJECT;
use super::iteration_runtime::{Context as Control, NativeIterationExecutionError};
use super::memory::{NativeWordStorage, transport_bytes};
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

#[derive(Debug)]
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
struct Context<'a> {
    control: Control<'a, NativeProgramDescriptor>,
    // Sources are cheap shared roots from the arena; drop collection state
    // first, while that arena still owns their prepaid retirement obligation.
    collections: Option<collection::State>,
    arena: NativeValueArena<'a>,
    library: NativeLibraryWorkBudget,
    limits: MemoryBudgetLimits,
    result: *mut u64,
    consumed: bool,
    output_cursor: usize,
    error: Option<NativeProgramExecutionError>,
}
impl Context<'_> {
    fn needs_move(&self, site: usize) -> Option<(NativeMatchValue, NativeMatchValue)> {
        let plan = &self.control.descriptor.plan;
        let pair = match plan.blocks()[site].action() {
            NativeIterationAction::Scalar(NativeMatchAction::Copy { source, output }) => {
                (*source, *output)
            }
            NativeIterationAction::Initialize { iteration } => {
                let i = plan.iterations()[*iteration as usize].as_iteration()?;
                (i.seed(), i.carrier())
            }
            NativeIterationAction::Latch { iteration } => {
                let i = plan.iterations()[*iteration as usize].as_iteration()?;
                (i.next(), i.carrier())
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
        args: [u64; 2],
    ) -> Result<Option<u64>, NativeProgramExecutionError> {
        if self.control.previous.map(u64::from) != Some(site) {
            return Err(invalid("helper outside exact admitted visit"));
        }
        let site = site as usize;
        let descriptor = self.control.descriptor;
        if operation == LIBRARY {
            if self.consumed {
                return Err(invalid("duplicate library helper"));
            }
            let NativeIterationAction::LibraryCall {
                operands, output, ..
            } = descriptor.plan.blocks()[site].action()
            else {
                return Err(invalid("library helper outside selected call"));
            };
            if operands.len() > args.len() {
                return Err(invalid("library helper arity"));
            }
            let mut inputs: [Option<Value>; 2] = [None, None];
            for (index, &operand) in operands.iter().enumerate() {
                inputs[index] = Some(
                    if descriptor.plan.values()[operand.get() as usize]
                        .scalar_kind()
                        .is_some()
                    {
                        self.scalar(operand, args[index])?
                    } else {
                        self.arena
                            .copy_current(operand.get(), &mut self.control.work)?
                    },
                );
            }
            let call = descriptor.calls[site]
                .as_ref()
                .ok_or_else(|| invalid("missing exact prepared selection"))?;
            let mut result = None;
            call.invoke_current(
                &inputs[..operands.len()],
                &mut result,
                self.limits,
                &self.library,
            )
            .map_err(|error| NativeProgramExecutionError::Library {
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
            let out = descriptor
                .plan
                .outputs()
                .get(self.output_cursor)
                .ok_or_else(|| invalid("extra typed output"))?;
            let value = if out.ty().scalar_kind().is_some() {
                self.scalar(out.value(), args[0])?
            } else {
                self.arena
                    .copy_current(out.value().get(), &mut self.control.work)?
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
            && matches!(
                descriptor.plan.iterations()[iteration as usize],
                mech_engine::native_artifact::NativeControlLoop::Collection(_)
            )
        {
            self.collections
                .as_mut()
                .ok_or_else(|| invalid("collection exit without state"))?
                .release_source(iteration)?;
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
}
unsafe extern "C" fn admit(context: *mut c_void, site: u64) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: execute retains this invocation context and disjoint backing for
    // the synchronous private entry; no pointer escapes the call.
    let context = unsafe { &mut *context.cast::<Context<'_>>() };
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
    result: *mut u64,
) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: the private entry receives only execute's retained context.
    let context = unsafe { &mut *context.cast::<Context<'_>>() };
    if context.error.is_some() {
        return STATUS_ADMISSION;
    }
    let outcome = if result.is_null() || result != context.result {
        Err(invalid("helper result pointer mismatch"))
    } else {
        context.helper(site, operation, [first, second])
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
pub(super) fn execute<F: FnOnce(*mut Frame) -> u64>(
    descriptor: &NativeProgramDescriptor,
    entry: F,
    inputs: &[Value],
    outputs: &mut [Option<Value>],
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    let plan = &descriptor.plan;
    if inputs.len() != plan.inputs().len() {
        return Err(NativeExecutionError::InvalidInputs {
            expected: plan.inputs().len(),
            actual: inputs.len(),
        }
        .into());
    }
    if outputs.len() != plan.outputs().len() {
        return Err(NativeExecutionError::InvalidOutputs {
            expected: plan.outputs().len(),
            actual: outputs.len(),
        }
        .into());
    }
    if descriptor.stack > limits.execution.max_native_stack_bytes {
        return Err(NativeExecutionError::StackLimit {
            required: descriptor.stack,
            limit: limits.execution.max_native_stack_bytes,
        }
        .into());
    }
    let caps = limits.library;
    if [
        caps.max_output_elements,
        caps.max_output_bytes,
        caps.max_temporary_bytes,
        caps.max_cloned_bytes,
        caps.max_retained_nodes,
        caps.max_compute_work,
        caps.max_comparison_work,
    ]
    .iter()
    .any(Option::is_none)
    {
        return Err(invalid(
            "native program requires seven finite library dimensions",
        ));
    }
    let locals = plan
        .values()
        .len()
        .checked_add(plan.iterations().len())
        .ok_or_else(|| invalid("local count overflow"))?;
    let progress = plan
        .iterations()
        .len()
        .checked_mul(2)
        .ok_or_else(|| invalid("progress count overflow"))?;
    let words = inputs
        .len()
        .checked_add(locals)
        .and_then(|n| n.checked_add(progress))
        .and_then(|n| n.checked_add(outputs.len()))
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| invalid("transport count overflow"))?;
    let word_bytes = transport_bytes(words)?;
    let arena_bytes = amount(
        descriptor.types.len(),
        (core::mem::size_of::<Option<Value>>() + 8) as u64,
        0,
    )?;
    let bytes = word_bytes
        .checked_add(arena_bytes)
        .and_then(|n| n.checked_add(collection::State::metadata_bytes(plan).ok()?))
        .ok_or_else(|| invalid("combined transport overflow"))?;
    if bytes > limits.execution.max_transport_bytes {
        return Err(NativeExecutionError::TransportLimit {
            required: bytes,
            limit: limits.execution.max_transport_bytes,
        }
        .into());
    }
    let mut work = NativeWorkMeter::new(limits.execution.max_work)?;
    work.charge(NativeExecutionPhase::Setup, amount(words, 4, 256)?)?;
    let library = work.shared_library_budget(
        NativeExecutionPhase::Setup,
        caps.max_comparison_work.unwrap(),
        memory,
    )?;
    let mut storage =
        NativeWordStorage::new(words, limits.execution.max_transport_bytes, Some(memory))?;
    let mut arena = NativeValueArena::new(
        &descriptor.types,
        limits.execution.max_transport_bytes,
        memory,
        &mut work,
    )?;
    // Validate/prepay prior roots before entry; publication itself cannot fail
    // halfway through replacing caller-visible output slots.
    for (value, ty) in outputs
        .iter()
        .zip(plan.outputs())
        .filter_map(|(value, ty)| value.as_ref().map(|v| (v, ty.ty())))
    {
        if value.schema() != ty.schema() || value.schema_key() != ty.schema_key() {
            return Err(NativeValueError::TypeMismatch.into());
        }
        let retained = value
            .memory_budget_retained_bytes(memory)
            .ok_or(NativeValueError::UnadmittedPayload)?;
        let retirement = retained
            .checked_mul(4)
            .and_then(|n| n.checked_add(32))
            .ok_or_else(|| invalid("output retirement overflow"))?;
        work.charge(NativeExecutionPhase::Setup, retirement)?;
    }
    for (value, input) in inputs.iter().zip(plan.inputs()) {
        arena.put(input.value().get(), value, &mut work)?;
    }
    for (constant, &bound) in plan.constants().iter().zip(&descriptor.constant_bytes) {
        work.charge(
            NativeExecutionPhase::Setup,
            bound
                .checked_mul(8)
                .and_then(|n| n.checked_add(128))
                .ok_or_else(|| invalid("constant admission work overflow"))?,
        )?;
        let mut reservation = memory.reserve_capacity(bound)?;
        // A raw constant may already have arbitrarily many foreign imports.
        // Its precharged full bound pays for an independent wrapper; never
        // search those other accounts under this invocation's finite work cap.
        let value = constant
            .constant()
            .clone()
            .into_independent_memory_budget(&mut reservation)?;
        arena.put(constant.value().get(), &value, &mut work)?;
    }
    let collections = collection::State::new(descriptor, caps, memory, &mut work)?;
    let (input_words, rest) = storage.values_mut().split_at_mut(inputs.len());
    let (values, rest) = rest.split_at_mut(locals);
    let (progress_words, rest) = rest.split_at_mut(progress);
    let (output_words, result) = rest.split_at_mut(outputs.len());
    for ((word, value), input) in input_words.iter_mut().zip(inputs).zip(plan.inputs()) {
        if let Some(kind) = input.ty().scalar_kind() {
            *word = bits(value, kind)?;
        }
    }
    let input_pointer = input_words.as_ptr();
    let local_pointer = values.as_mut_ptr();
    let output_pointer = output_words.as_mut_ptr();
    let result_pointer = result.as_mut_ptr();
    let mut context = Context {
        control: Control {
            descriptor,
            progress: progress_words,
            current: None,
            previous: None,
            completed: false,
            work,
            visits: 0,
            error: None,
        },
        collections,
        arena,
        library,
        limits: caps,
        result: result_pointer,
        consumed: false,
        output_cursor: 0,
        error: None,
    };
    let context_pointer = (&mut context as *mut Context<'_>).cast::<c_void>();
    let mut frame = Frame {
        base: NativeEntryFrame {
            version: VERSION,
            input_count: inputs.len() as u64,
            local_count: locals as u64,
            output_count: outputs.len() as u64,
            inputs: input_pointer,
            locals: local_pointer,
            outputs: output_pointer,
            context: context_pointer,
            admit_next: admit,
            error_node: NO_NODE,
        },
        helper,
        result: result_pointer,
    };
    let status = entry(&mut frame);
    if let Some(error) = context.error.take() {
        return Err(error);
    }
    let f = &frame.base;
    if f.version != VERSION
        || f.input_count != inputs.len() as u64
        || f.local_count != locals as u64
        || f.output_count != outputs.len() as u64
        || f.inputs != input_pointer
        || f.locals != local_pointer
        || f.outputs != output_pointer
        || f.context != context_pointer
        || f.admit_next as usize != admit as *const () as usize
        || frame.helper as usize != helper as *const () as usize
        || frame.result != result_pointer
    {
        return Err(invalid("program entry changed immutable frame"));
    }
    match status {
        STATUS_OK => {
            if !context.control.completed
                || f.error_node != NO_NODE
                || context.output_cursor != outputs.len()
                || context
                    .control
                    .previous
                    .is_some_and(|site| context.action_required(site as usize) && !context.consumed)
            {
                return Err(invalid("program omitted atomic completion"));
            }
        }
        STATUS_ARITHMETIC | STATUS_REJECT => {
            let id = context
                .control
                .previous
                .ok_or_else(|| invalid("program failure before visit"))?;
            let site = &descriptor.visits[id as usize];
            if f.error_node != id as u64 {
                return Err(invalid("program failure origin mismatch"));
            }
            return Err(if status == STATUS_ARITHMETIC && site.arithmetic {
                NativeIterationExecutionError::Arithmetic {
                    origin: site.origin,
                }
                .into()
            } else if status == STATUS_REJECT && site.reject {
                NativeIterationExecutionError::Unmatched {
                    origin: site.origin,
                }
                .into()
            } else {
                invalid("program failure status outside matching action")
            });
        }
        _ => return Err(NativeExecutionError::InvalidStatus { status }.into()),
    }
    context
        .arena
        .publish_range(plan.values().len(), outputs, &mut context.control.work)?;
    Ok(NativeProgramExecutionReport {
        work: context.control.work.used(),
        comparison_work: context.library.used().1,
        visits: context.control.visits,
        transport_bytes: bytes,
    })
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
    fn reach_call(frame: &mut Frame, descriptor: &NativeProgramDescriptor) -> u32 {
        let mut site = descriptor.plan.entry();
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
        for mode in 0..5 {
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
                    // SAFETY: arguments remain within the retained ABI5 boundary;
                    // wrong metadata/pointer is rejected before dereference.
                    let status = unsafe {
                        (frame.helper)(frame.base.context, actual_site, operation, 0, 0, result)
                    };
                    if mode == 3 {
                        assert_eq!(status, STATUS_OK);
                        // SAFETY: same live context/result lane, deliberate repeat.
                        unsafe {
                            (frame.helper)(frame.base.context, actual_site, operation, 0, 0, result)
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
                    let admit = |site| {
                        // SAFETY: the same live private callback/context; only
                        // visit metadata is varied, never pointer ownership.
                        unsafe { (frame.base.admit_next)(frame.base.context, site as u64) }
                    };
                    let invoke = |site, operation, result| {
                        // SAFETY: retained private helper/context; a deliberately
                        // wrong result pointer is checked before dereference.
                        unsafe {
                            (frame.helper)(frame.base.context, site as u64, operation, 0, 0, result)
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
                    let mut site = descriptor.plan.entry();
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
    fn abi5_keeps_the_frozen_scalar_prefix_and_disjoint_helper_lanes() {
        assert_eq!(VERSION, 5);
        assert_eq!(core::mem::offset_of!(Frame, base), 0);
        assert_eq!(
            core::mem::offset_of!(Frame, helper),
            core::mem::size_of::<NativeEntryFrame>()
        );
        assert_eq!(
            core::mem::offset_of!(Frame, result),
            core::mem::size_of::<NativeEntryFrame>() + 8
        );
    }
}
