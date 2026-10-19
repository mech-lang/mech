//! Owning two-phase state for ONE generated CFG invocation. Raw frame pointers
//! are reconstructed from admitted owned backing for each synchronous entry;
//! neither caller pointers nor self-referential borrowed state are retained.
use super::super::program_abi::{PHASE_ACTIVATION, PHASE_TURN, STATUS_ACTIVATED};
use super::*;

struct OwnedState<'a> {
    // Preserve collection-before-arena retirement ordering.
    collections: Option<collection::State>,
    arena: NativeValueArena<'a>,
    call_inputs: NativeTypedStorage<Option<Value>>,
    library: NativeLibraryWorkBudget,
    work: NativeWorkMeter,
    current: Option<u32>,
    previous: Option<u32>,
    completed: bool,
    visits: u64,
    consumed: bool,
    output_cursor: usize,
}

pub(super) struct Prepared<'a> {
    descriptor: &'a NativeProgramDescriptor,
    state: Option<OwnedState<'a>>,
    storage: NativeWordStorage,
    memory: ManagedMemoryBudget,
    limits: NativeProgramExecutionLimits,
    locals: usize,
    progress: usize,
    bytes: u64,
    activated: bool,
}

impl<'a> Prepared<'a> {
    pub(super) fn new(
        descriptor: &'a NativeProgramDescriptor,
        limits: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
    ) -> Result<Self, NativeProgramExecutionError> {
        let plan = &descriptor.plan;
        if plan.activation_boundary().is_none() {
            return Err(invalid(
                "managed execution has no certified activation boundary",
            ));
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
        let words = plan
            .transport_input_count()
            .checked_add(locals)
            .and_then(|n| n.checked_add(progress))
            .and_then(|n| n.checked_add(plan.transport_output_count()))
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_add(descriptor.max_library_inputs.max(1)))
            .ok_or_else(|| invalid("transport count overflow"))?;
        let bytes = transport_bytes(words)?
            .checked_add(amount(
                descriptor.types.len(),
                (core::mem::size_of::<Option<Value>>() + 8) as u64,
                0,
            )?)
            .and_then(|n| {
                n.checked_add(
                    amount(
                        descriptor.max_library_inputs,
                        core::mem::size_of::<Option<Value>>() as u64,
                        0,
                    )
                    .ok()?,
                )
            })
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
        work.charge(
            NativeExecutionPhase::Setup,
            amount(
                descriptor.max_library_inputs,
                (core::mem::size_of::<Option<Value>>() as u64) * 8 + 64,
                4096,
            )?,
        )?;
        let library = work.shared_library_budget(
            NativeExecutionPhase::Setup,
            caps.max_comparison_work.unwrap(),
            memory,
        )?;
        let storage =
            NativeWordStorage::new(words, limits.execution.max_transport_bytes, Some(memory))?;
        let mut arena = NativeValueArena::new(
            &descriptor.types,
            limits.execution.max_transport_bytes,
            memory,
            &mut work,
        )?;
        let call_inputs = NativeTypedStorage::new_values(
            descriptor.max_library_inputs,
            limits.execution.max_transport_bytes,
            memory,
        )?;
        // Constants and their retirement are funded only once, before any
        // generated activation. No fresh Values or observations are touched.
        for (constant, &bound) in plan.constants().iter().zip(&descriptor.constant_bytes) {
            work.charge(
                NativeExecutionPhase::Setup,
                bound
                    .checked_mul(8)
                    .and_then(|n| n.checked_add(128))
                    .ok_or_else(|| invalid("constant admission work overflow"))?,
            )?;
            let mut reservation = memory.reserve_capacity(bound)?;
            let value = constant
                .constant()
                .clone()
                .into_independent_memory_budget(&mut reservation)?;
            arena.put(constant.value().get(), &value, &mut work)?;
        }
        let collections = collection::State::new(descriptor, caps, memory, &mut work)?;
        Ok(Self {
            descriptor,
            storage,
            memory: memory.clone(),
            limits,
            locals,
            progress,
            bytes,
            activated: false,
            state: Some(OwnedState {
                collections,
                arena,
                call_inputs,
                library,
                work,
                current: None,
                previous: None,
                completed: false,
                visits: 0,
                consumed: false,
                output_cursor: 0,
            }),
        })
    }

    pub(super) fn activate<F: FnMut(*mut Frame) -> u64>(
        &mut self,
        entry: &mut F,
    ) -> Result<(), NativeProgramExecutionError> {
        if self.activated {
            return Err(invalid("activation already completed"));
        }
        self.run(entry, PHASE_ACTIVATION)?;
        self.activated = true;
        Ok(())
    }

    fn import_turn(
        &mut self,
        inputs: &[Value],
        observations: &[Value],
        outputs: &[Option<Value>],
    ) -> Result<(), NativeProgramExecutionError> {
        if !self.activated {
            return Err(invalid("turn before generated activation"));
        }
        let descriptor = self.descriptor;
        let plan = &descriptor.plan;
        if inputs.len() != plan.inputs().len() {
            return Err(NativeExecutionError::InvalidInputs {
                expected: plan.inputs().len(),
                actual: inputs.len(),
            }
            .into());
        }
        if observations.len() != plan.observations().len() {
            return Err(invalid(
                "captured observation count differs from exact native plan",
            ));
        }
        if outputs.len() != plan.transport_output_count() {
            return Err(NativeExecutionError::InvalidOutputs {
                expected: plan.transport_output_count(),
                actual: outputs.len(),
            }
            .into());
        }
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        // Old caller-visible roots are not examined until closed activation
        // has succeeded. Publication still cannot fail partway through.
        for (index, value) in outputs
            .iter()
            .enumerate()
            .filter_map(|(index, value)| value.as_ref().map(|v| (index, v)))
        {
            let ty = plan.transport_output_type(index);
            if value.schema() != ty.schema() || value.schema_key() != ty.schema_key() {
                return Err(NativeValueError::TypeMismatch.into());
            }
            let retained = value
                .memory_budget_retained_bytes(&self.memory)
                .ok_or(NativeValueError::UnadmittedPayload)?;
            state.work.charge(
                NativeExecutionPhase::Setup,
                retained
                    .checked_mul(4)
                    .and_then(|n| n.checked_add(32))
                    .ok_or_else(|| invalid("output retirement overflow"))?,
            )?;
        }
        for (index, value) in inputs.iter().enumerate() {
            state
                .arena
                .put(plan.inputs()[index].value().get(), value, &mut state.work)?;
        }
        for (ordinal, value) in observations.iter().enumerate() {
            let index = inputs
                .len()
                .checked_add(ordinal)
                .ok_or_else(|| invalid("observation ordinal overflow"))?;
            let ty = plan.transport_input_type(index);
            let ValueData::String(text) = value.data() else {
                return Err(invalid("captured native observation is not String"));
            };
            if ty.definition().body() != &mech_core::SchemaBody::String
                || value.schema_key() != ty.schema_key()
            {
                return Err(invalid("captured native observation schema differs"));
            }
            let bound = amount(text.len(), 4, descriptor.scalar_bytes)?;
            state.work.charge(
                NativeExecutionPhase::Setup,
                bound
                    .checked_mul(8)
                    .and_then(|n| n.checked_add(128))
                    .ok_or_else(|| invalid("observation import work overflow"))?,
            )?;
            let mut reservation = self.memory.reserve_capacity(bound)?;
            let shape = ty
                .definition()
                .instantiate_shape(Box::new([]))
                .map_err(|error| NativeProgramExecutionError::Snapshot(error.into()))?;
            let rebound = value
                .rebind_with_context(
                    ty.schema(),
                    &shape,
                    &SnapshotValidationContext::with_shared_schemas(&descriptor.schemas),
                )
                .map_err(NativeProgramExecutionError::Snapshot)?;
            let owned = rebound.into_independent_memory_budget(&mut reservation)?;
            state.arena.put(
                plan.transport_input_value(index).get(),
                &owned,
                &mut state.work,
            )?;
        }
        // This is typed transport only, not expression evaluation. Closed
        // locals/constants remain untouched; only real fresh input lanes enter.
        let (input_words, rest) = self
            .storage
            .values_mut()
            .split_at_mut(plan.transport_input_count());
        let values = &mut rest[..self.locals];
        for (index, value) in inputs.iter().enumerate() {
            if let Some(kind) = plan.inputs()[index].ty().scalar_kind() {
                let word = bits(value, kind)?;
                input_words[index] = word;
                values[plan.inputs()[index].value().get() as usize] = word;
            }
        }
        Ok(())
    }

    pub(super) fn finish<F: FnMut(*mut Frame) -> u64>(
        mut self,
        entry: &mut F,
        inputs: &[Value],
        observations: &[Value],
        outputs: &mut [Option<Value>],
    ) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
        self.import_turn(inputs, observations, outputs)?;
        self.run(entry, PHASE_TURN)?;
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        state.arena.publish_range(
            self.descriptor.plan.values().len(),
            outputs,
            &mut state.work,
        )?;
        Ok(NativeProgramExecutionReport {
            work: state.work.used(),
            comparison_work: state.library.used().1,
            visits: state.visits,
            transport_bytes: self.bytes,
        })
    }

    fn run<F: FnMut(*mut Frame) -> u64>(
        &mut self,
        entry: &mut F,
        phase: u64,
    ) -> Result<(), NativeProgramExecutionError> {
        let descriptor = self.descriptor;
        let plan = &descriptor.plan;
        let boundary = plan
            .activation_boundary()
            .ok_or_else(|| invalid("missing certified activation boundary"))?;
        let mut state = self
            .state
            .take()
            .ok_or_else(|| invalid("consumed native state"))?;
        // Both entry prologues and checkpoint verification are paid on this
        // SAME ledger before entering native code or scanning progress arrays.
        let transition_work = amount(plan.iterations().len(), 32, 512)?
            .checked_add(amount(plan.collections().len(), 32, 0)?)
            .ok_or_else(|| invalid("native phase work overflow"))?;
        state
            .work
            .charge(NativeExecutionPhase::Setup, transition_work)?;
        let (input_words, rest) = self
            .storage
            .values_mut()
            .split_at_mut(plan.transport_input_count());
        let (values, rest) = rest.split_at_mut(self.locals);
        let (progress_words, rest) = rest.split_at_mut(self.progress);
        let (output_words, rest) = rest.split_at_mut(plan.transport_output_count());
        let (result, arguments) = rest.split_at_mut(1);
        let input_pointer = input_words.as_ptr();
        let local_pointer = values.as_mut_ptr();
        let output_pointer = output_words.as_mut_ptr();
        let result_pointer = result.as_mut_ptr();
        let argument_pointer = arguments.as_mut_ptr();
        let mut context = Context {
            control: Control {
                descriptor,
                progress: progress_words,
                current: state.current,
                previous: state.previous,
                completed: state.completed,
                work: state.work,
                visits: state.visits,
                error: None,
            },
            collections: state.collections,
            arena: state.arena,
            library: state.library,
            limits: self.limits.library,
            result: result_pointer,
            arguments: argument_pointer,
            argument_capacity: descriptor.max_library_inputs,
            call_inputs: state.call_inputs,
            consumed: state.consumed,
            output_cursor: state.output_cursor,
            error: None,
            activation_phase: phase == PHASE_ACTIVATION,
        };
        let context_pointer = (&mut context as *mut Context<'_, '_>).cast::<c_void>();
        let mut frame = Frame {
            base: NativeEntryFrame {
                version: VERSION,
                input_count: plan.transport_input_count() as u64,
                local_count: self.locals as u64,
                output_count: plan.transport_output_count() as u64,
                inputs: input_pointer,
                locals: local_pointer,
                outputs: output_pointer,
                context: context_pointer,
                admit_next: admit,
                error_node: NO_NODE,
            },
            helper,
            result: result_pointer,
            arguments: argument_pointer,
            argument_capacity: descriptor.max_library_inputs as u64,
            phase,
        };
        let status = entry(&mut frame);
        if let Some(error) = context.error.take() {
            return Err(error);
        }
        let f = &frame.base;
        if f.version != VERSION
            || f.input_count != plan.transport_input_count() as u64
            || f.local_count != self.locals as u64
            || f.output_count != plan.transport_output_count() as u64
            || f.inputs != input_pointer
            || f.locals != local_pointer
            || f.outputs != output_pointer
            || f.context != context_pointer
            || f.admit_next as usize != admit as *const () as usize
            || frame.helper as usize != helper as *const () as usize
            || frame.result != result_pointer
            || frame.arguments != argument_pointer
            || frame.argument_capacity != descriptor.max_library_inputs as u64
            || frame.phase != phase
        {
            return Err(invalid("program entry changed immutable frame"));
        }
        if phase == PHASE_ACTIVATION && status == STATUS_ACTIVATED {
            if f.error_node != u64::from(boundary)
                || context.control.completed
                || context.control.current.is_some()
                || context
                    .control
                    .progress
                    .chunks_exact(2)
                    .any(|value| value[1] != 0)
                || context.output_cursor != 0
                || context
                    .collections
                    .as_ref()
                    .is_some_and(|state| !state.is_complete())
                || context.control.previous.is_none_or(|previous| {
                    !descriptor.visits[previous as usize]
                        .successors()
                        .contains(&Some(boundary))
                        || (context.action_required(previous as usize) && !context.consumed)
                })
            {
                return Err(invalid("native entry omitted exact activation checkpoint"));
            }
        } else {
            match status {
                STATUS_OK if phase == PHASE_TURN => {
                    if !context.control.completed
                        || f.error_node != NO_NODE
                        || context.output_cursor != plan.transport_output_count()
                        || context.control.previous.is_some_and(|site| {
                            context.action_required(site as usize) && !context.consumed
                        })
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
                    if f.error_node != u64::from(id) {
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
        }
        self.state = Some(OwnedState {
            collections: context.collections,
            arena: context.arena,
            call_inputs: context.call_inputs,
            library: context.library,
            work: context.control.work,
            current: context.control.current,
            previous: context.control.previous,
            completed: context.control.completed,
            visits: context.control.visits,
            consumed: context.consumed,
            output_cursor: context.output_cursor,
        });
        Ok(())
    }
}

#[cfg(feature = "canonical-native-program-host-runtime")]
pub(in crate::native_artifact) enum CheckedEntry {
    #[cfg(feature = "canonical-native-program")]
    Jit(super::super::program_abi::Entry),
    Linked(unsafe extern "C" fn(*mut c_void) -> u64),
}
#[cfg(feature = "canonical-native-program-host-runtime")]
impl CheckedEntry {
    fn invoke(&mut self, frame: *mut Frame) -> u64 {
        // SAFETY: only the JIT/linked owner facades construct this private
        // provenance, and the session borrows that owner for its full lifetime.
        unsafe {
            match self {
                #[cfg(feature = "canonical-native-program")]
                Self::Jit(entry) => entry(frame),
                Self::Linked(entry) => entry(frame.cast::<c_void>()),
            }
        }
    }
}

/// Opaque owning pre-capture activation result. Not Clone: resume consumes it,
/// retaining one arena/work account and the original checked module lifetime.
#[cfg(feature = "canonical-native-program-host-runtime")]
pub struct NativeProgramExecutionSession<'a> {
    prepared: Prepared<'a>,
    entry: CheckedEntry,
    invocation: mech_runtime::CanonicalNativeResourceInvocation,
}
#[cfg(feature = "canonical-native-program-host-runtime")]
impl<'a> NativeProgramExecutionSession<'a> {
    pub(super) fn prepare(
        descriptor: &'a NativeProgramDescriptor,
        mut entry: CheckedEntry,
        admission: mech_runtime::CanonicalNativeResourceAdmission<'_>,
        limits: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
    ) -> Result<Self, NativeProgramExecutionError> {
        verify_hosted_plan(descriptor, admission.program_revision(), admission.plan())?;
        let invocation = admission.invocation();
        let mut prepared = Prepared::new(descriptor, limits, memory)?;
        prepared.activate(&mut |frame| entry.invoke(frame))?;
        Ok(Self {
            prepared,
            entry,
            invocation,
        })
    }
    pub fn resume_hosted_into(
        self,
        turn: mech_runtime::CanonicalNativeResourceTurn<'_>,
        inputs: &[Value],
        outputs: &mut [Option<Value>],
    ) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
        if turn.invocation() != self.invocation {
            return Err(invalid(
                "native activation session belongs to another admitted transaction",
            ));
        }
        verify_hosted_turn(self.prepared.descriptor, &turn)?;
        let Self {
            prepared,
            mut entry,
            ..
        } = self;
        prepared.finish(
            &mut |frame| entry.invoke(frame),
            inputs,
            turn.observations(),
            outputs,
        )
    }
}
