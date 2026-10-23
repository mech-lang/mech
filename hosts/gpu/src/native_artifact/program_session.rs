//! Owning two-phase state for ONE generated CFG invocation. Raw frame pointers
//! are reconstructed from admitted owned backing for each synchronous entry;
//! neither caller pointers nor self-referential borrowed state are retained.
use super::super::program_abi::{
    PHASE_ACTIVATION, PHASE_CAPTURE_TAIL, PHASE_PREFIX, PHASE_TURN, STATUS_ACTIVATED,
    STATUS_INPUT_PENDING, STATUS_INPUT_SCANNED,
};
use super::*;

#[path = "program_call_cache_allocation.rs"]
mod call_cache_allocation;

#[path = "program_capture.rs"]
mod capture;
#[cfg(feature = "canonical-native-program-host-runtime")]
#[path = "program_capture_admission.rs"]
mod capture_admission;
#[cfg(feature = "canonical-native-program-host-runtime")]
#[path = "program_capture_host.rs"]
mod capture_host;

#[cfg(test)]
#[path = "program_observation_tests.rs"]
mod observation_tests;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_call_cache_tests.rs"]
mod call_cache_tests;
#[cfg(all(test, feature = "canonical-native-program-host"))]
#[path = "program_capture_host_tests.rs"]
mod capture_host_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_capture_tests.rs"]
mod capture_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_identity_inventory_tests.rs"]
mod identity_inventory_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_prefix_tests.rs"]
mod prefix_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_prefix_window_tests.rs"]
mod prefix_window_tests;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_output_retirement_tests.rs"]
mod output_retirement_tests;

/// Validate and fund retirement of the caller's existing output roots before
/// native publication. The shared account remains available even if a failed
/// speculative scanner has already retired its non-runnable execution state.
fn admit_output_retirement(
    descriptor: &NativeProgramDescriptor,
    outputs: &[Option<Value>],
    memory: &ManagedMemoryBudget,
    library: &NativeLibraryWorkBudget,
) -> Result<(), NativeProgramExecutionError> {
    let plan = &descriptor.plan;
    if outputs.len() != plan.transport_output_count() {
        return Err(NativeExecutionError::InvalidOutputs {
            expected: plan.transport_output_count(),
            actual: outputs.len(),
        }
        .into());
    }
    for (index, value) in outputs
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.as_ref().map(|value| (index, value)))
    {
        let ty = plan.transport_output_type(index);
        if value.schema() != ty.schema() || value.schema_key() != ty.schema_key() {
            return Err(NativeValueError::TypeMismatch.into());
        }
        let retained = value
            .memory_budget_retained_bytes(memory)
            .ok_or(NativeValueError::UnadmittedPayload)?;
        let debit = retained
            .checked_mul(4)
            .and_then(|bytes| bytes.checked_add(32))
            .ok_or_else(|| invalid("output retirement overflow"))?;
        library.charge(debit, 0).map_err(|error| match error {
            mech_core::ResidentKernelError::ResourceLimit {
                dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                required,
                limit,
                ..
            } => NativeExecutionError::WorkLimit {
                phase: NativeExecutionPhase::Setup,
                required,
                limit,
            },
            error => NativeExecutionError::LibraryWork {
                phase: NativeExecutionPhase::Setup,
                error,
            },
        })?;
    }
    Ok(())
}

/// Native ownership admission for the selected zero-parameter String lane.
/// Immutable payload sharing never substitutes for full capacity admission.
fn import_string_observation(
    value: &Value,
    schema: mech_core::SchemaId,
    schemas: &std::sync::Arc<mech_core::SchemaTable>,
    target_schema_bytes: u64,
    memory: &ManagedMemoryBudget,
    work: &mut NativeWorkMeter,
) -> Result<(Value, mech_core::ManagedMemoryReservation), NativeProgramExecutionError> {
    work.check_memory_budget(memory)?;
    let ValueData::String(text) = value.data() else {
        return Err(invalid("captured native observation is not String"));
    };
    let source_schemas = value
        .schemas()
        .ok_or_else(|| invalid("captured String has no retained schema context"))?;
    let source_schema = value
        .validate_against(&source_schemas)
        .map_err(NativeProgramExecutionError::Snapshot)?;
    let target = schemas
        .entry(schema)
        .ok_or_else(|| invalid("captured native observation target schema is missing"))?;
    if source_schema.body() != &mech_core::SchemaBody::String
        || target.schema().body() != &mech_core::SchemaBody::String
        || !source_schema.dimension_parameters().is_empty()
        || !target.schema().dimension_parameters().is_empty()
        || !value.shape().parameter_values().is_empty()
        || value.schema_key() != target.key()
    {
        return Err(invalid("captured native observation schema differs"));
    }
    // Bound discovery itself consumes this SAME invocation ledger. The
    // canonical traversal stops before a denied shallow contribution's children.
    let source_schema_bytes = source_schemas
        .clone_allocation_bound_bytes_with_precharge(|bytes| {
            let debit = bytes
                .checked_mul(8)
                .ok_or_else(|| invalid("observation schema measurement work overflows"))?;
            work.charge(NativeExecutionPhase::Setup, debit)
                .map_err(NativeProgramExecutionError::from)
        })
        .map_err(|error| match error {
            mech_core::SchemaAllocationBoundError::Precharge(error) => error,
            mech_core::SchemaAllocationBoundError::ArithmeticOverflow => {
                invalid("observation source schema ownership bound overflows")
            }
        })?;
    let payload_bytes =
        u64::try_from(text.len()).map_err(|_| invalid("observation String length exceeds u64"))?;
    let footprint =
        mech_core::snapshot::prospective_string_value_footprint(0, false, 1, payload_bytes)
            .map_err(|_| invalid("observation String ownership bound overflows"))?;
    // The canonical owner bound includes one schema Arc header. Admit both
    // full source/target arenas plus the second header, even when they alias.
    let second_schema_header = mech_core::SchemaTable::clone_metadata_allocation_bound_bytes()
        .checked_sub(core::mem::size_of::<mech_core::SchemaTable>() as u64)
        .ok_or_else(|| invalid("observation schema owner layout is invalid"))?;
    let bound = [
        footprint.retained_bytes,
        Value::canonical_owner_allocation_bytes(),
        source_schema_bytes,
        target_schema_bytes,
        second_schema_header,
        Value::memory_budget_claim_metadata_bytes(),
    ]
    .into_iter()
    .try_fold(0_u64, |total, bytes| {
        total
            .checked_add(bytes)
            .ok_or_else(|| invalid("observation import ownership bound overflows"))
    })?;
    // Measurement already paid 8 * source_schema_bytes; the cumulative total
    // remains exactly 8 * bound + 128. Existing arena retirement is separate.
    let remaining = bound
        .checked_sub(source_schema_bytes)
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| n.checked_add(128))
        .ok_or_else(|| invalid("observation import work overflows"))?;
    work.charge(NativeExecutionPhase::Setup, remaining)?;
    let mut reservation = memory.reserve_capacity(bound)?;
    // These checks select the allocation-free canonical String sharing path.
    // The context's target owner is cloned only after full admission above.
    let rebound = value
        .rebind_with_context(
            schema,
            value.shape(),
            &SnapshotValidationContext::with_shared_schemas(schemas),
        )
        .map_err(NativeProgramExecutionError::Snapshot)?;
    let owned = rebound.into_independent_memory_budget(&mut reservation)?;
    Ok((owned, reservation))
}

/// A private storage choice, not permission to visit or invoke a site.
/// The validated root-scope DAG visits a Once site at most once across the
/// SAME activation/turn invocation. Only a structurally owned loop can repeat.
pub(super) enum CallCacheEntry {
    Once,
    Repeated(NativePreparedLibraryCallCache),
}
impl CallCacheEntry {
    pub(super) fn invoke(
        &mut self,
        call: &mech_engine::native_artifact::NativePreparedLibraryCall,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, NativeCollectionInvocationError> {
        match self {
            Self::Once => call.invoke_current(inputs, output, limits, work),
            Self::Repeated(cache) => {
                call.invoke_current_cached(cache, inputs, output, limits, work)
            }
        }
    }
}

/// Empty per-site slots are native invocation metadata, not prebound calls.
/// Their complete concrete backing is admitted during setup. Once slots have
/// no cache owner; repeated slots separately admit lazy bound metadata.
pub(super) struct CallCacheLane {
    pub(super) entries: Box<[CallCacheEntry]>,
    _reservation: mech_core::ManagedMemoryReservation,
}
impl CallCacheLane {
    fn metadata_bytes(count: usize) -> Result<u64, NativeProgramExecutionError> {
        amount(
            count,
            (core::mem::size_of::<CallCacheEntry>() as u64) * 2,
            128,
        )
    }
    fn new(
        descriptor: &NativeProgramDescriptor,
        limit: u64,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Self, NativeProgramExecutionError> {
        let blocks = descriptor.plan.blocks();
        let count = blocks.len();
        let bytes = Self::metadata_bytes(count)?;
        if bytes > limit {
            return Err(NativeExecutionError::TransportLimit {
                required: bytes,
                limit,
            }
            .into());
        }
        work.charge(
            NativeExecutionPhase::Setup,
            bytes
                .checked_mul(8)
                .and_then(|n| n.checked_add(128))
                .ok_or_else(|| invalid("call cache setup work overflow"))?,
        )?;
        let reservation = memory.reserve_capacity(bytes)?;
        // NativeProgramDescriptor::prepare already ran BOTH canonical phase
        // and current-visit proofs. check_scope contracts each child loop to
        // initializer -> exit and rejects every remaining scope cycle. Thus
        // None is a once-only root DAG site, including nested Match arms;
        // Some covers counted, collection, tail and enclosing nested loops.
        // Phase resume retains the same previous/current/progress state and
        // cannot replay a root block. No new execution authority is created.
        let entries = call_cache_allocation::allocate(count, bytes, |ordinal| {
            let block = &blocks[ordinal];
            if block.iteration().is_none() {
                CallCacheEntry::Once
            } else {
                CallCacheEntry::Repeated(NativePreparedLibraryCallCache::default())
            }
        })?;
        Ok(Self {
            entries: entries.into_boxed_slice(),
            _reservation: reservation,
        })
    }
}

struct OwnedState<'a> {
    // Preserve collection-before-arena retirement ordering.
    collections: Option<collection::State>,
    line_matrices: line_matrices::State,
    arena: NativeValueArena<'a>,
    call_inputs: NativeTypedStorage<Option<Value>>,
    call_caches: CallCacheLane,
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
    prefix: Option<prefix::Input>,
    paused: Option<u32>,
    retention: Option<prefix::Retention>,
    window_work_seen: u64,
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
            .and_then(|n| n.checked_add(descriptor.scalar_aggregates.words()))
            .and_then(|n| n.checked_add(descriptor.borrowed_strings.words))
            .and_then(|n| n.checked_add(descriptor.closed_tails.words))
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
            .and_then(|n| n.checked_add(descriptor.retirements.words().len()))
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
            .and_then(|n| n.checked_add(collection::State::metadata_bytes(descriptor).ok()?))
            .and_then(|n| n.checked_add(CallCacheLane::metadata_bytes(plan.blocks().len()).ok()?))
            .and_then(|n| {
                n.checked_add(
                    line_matrices::State::metadata_bytes(descriptor.line_matrices.regions.len())
                        .ok()?,
                )
            })
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
        let mut storage =
            NativeWordStorage::new(words, limits.execution.max_transport_bytes, Some(memory))?;
        // The exact table occupies the disjoint final lane of this SAME
        // admitted typed arena; no new runtime plan or unowned metadata pool.
        // Initialization was prepaid above; fund its copy before touching it.
        work.charge(
            NativeExecutionPhase::Setup,
            amount(descriptor.retirements.words().len(), 16, 128)?,
        )?;
        storage.values_mut()[words - descriptor.retirements.words().len()..]
            .copy_from_slice(descriptor.retirements.words());
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
        let call_caches = CallCacheLane::new(
            descriptor,
            limits.execution.max_transport_bytes,
            memory,
            &mut work,
        )?;
        let line_matrices = line_matrices::State::new(
            descriptor,
            limits.execution.max_transport_bytes,
            memory,
            &mut work,
        )?;
        Ok(Self {
            descriptor,
            storage,
            memory: memory.clone(),
            limits,
            locals,
            progress,
            bytes,
            activated: false,
            prefix: None,
            paused: None,
            retention: None,
            window_work_seen: 0,
            state: Some(OwnedState {
                collections,
                line_matrices,
                arena,
                call_inputs,
                call_caches,
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
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        // Old caller-visible roots are not examined until closed activation
        // has succeeded. Publication still cannot fail partway through.
        admit_output_retirement(descriptor, outputs, &self.memory, &state.library)?;
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
            if ty.definition().body() != &mech_core::SchemaBody::String
                || !ty.definition().dimension_parameters().is_empty()
            {
                return Err(invalid("captured native observation schema differs"));
            }
            let (owned, _reservation) = import_string_observation(
                value,
                ty.schema(),
                &descriptor.schemas,
                descriptor.observation_schema_bytes,
                &self.memory,
                &mut state.work,
            )?;
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
        let imported = self.import_turn(inputs, observations, outputs);
        #[cfg(all(
            test,
            feature = "canonical-native-program",
            feature = "canonical-native-program-host-runtime"
        ))]
        if let Some(state) = &self.state {
            setup_work_diagnostic::checkpoint(state.work.used());
        }
        imported?;
        self.run(entry, PHASE_TURN)?;
        self.publish(outputs)
    }

    fn publish(
        mut self,
        outputs: &mut [Option<Value>],
    ) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
        if self.paused.is_some() || !self.state.as_ref().is_some_and(|state| state.completed) {
            return Err(invalid("publication before exact native completion"));
        }
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        let published = state.arena.publish_range(
            self.descriptor.plan.values().len(),
            outputs,
            &mut state.work,
        );
        #[cfg(all(
            test,
            feature = "canonical-native-program",
            feature = "canonical-native-program-host-runtime"
        ))]
        setup_work_diagnostic::checkpoint(state.work.used());
        published?;
        Ok(NativeProgramExecutionReport {
            work: state.work.used(),
            comparison_work: state.library.used().1,
            visits: state.visits,
            transport_bytes: self.bytes,
        })
    }

    // Exercise the real generated protocol before exposing any incremental
    // capture API. This test-only driver imports ONE genuine full snapshot;
    // neither a partial canonical Value nor a prefix-valued count is created.
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn run_prefix<F: FnMut(*mut Frame) -> u64>(
        &mut self,
        entry: &mut F,
        available: u64,
        eof: bool,
    ) -> Result<(), NativeProgramExecutionError> {
        self.run_prefix_window(entry, available, eof, None)
    }

    #[cfg(any(
        feature = "canonical-native-program-host-runtime",
        all(test, feature = "canonical-native-program")
    ))]
    fn run_prefix_window<F: FnMut(*mut Frame) -> u64>(
        &mut self,
        entry: &mut F,
        available: u64,
        eof: bool,
        window: Option<&mech_engine::native_artifact::NativeTextLineWindow<'_>>,
    ) -> Result<(), NativeProgramExecutionError> {
        if !self.activated
            || self.descriptor.prefix.is_none()
            || self.state.as_ref().is_none_or(|state| state.completed)
            || self
                .prefix
                .is_some_and(|prior| prior.eof || available < prior.available)
        {
            return Err(invalid("invalid, regressing or completed native prefix"));
        }
        if eof && self.prefix.is_none() {
            if let Some(window) = window {
                // Reject a false EOF extent, producer or account before even
                // the snapshot prelude can enter native code. Binding debits
                // the existing source account; it creates no new allowance.
                let state = self.state.as_ref().unwrap();
                prefix::validate_window(
                    self.descriptor,
                    Some(prefix::Input {
                        resume_site: self
                            .descriptor
                            .plan
                            .activation_boundary()
                            .ok_or_else(|| invalid("prefix lacks exact resume boundary"))?,
                        available,
                        eof,
                    }),
                    window,
                    &state.library,
                )?;
            }
            // A snapshot driver can receive EOF before its first window (in
            // particular for empty input). Execute the original source/length
            // visits and stop at the zero-available header before inspecting
            // their genuine final geometry. No count slot is initialized by
            // this pre-EOF entry, and no finished window is mislabelled pending.
            self.run_prefix_window(entry, 0, false, None)?;
        }
        let resume_site = self
            .paused
            .or(self.descriptor.plan.activation_boundary())
            .ok_or_else(|| invalid("prefix lacks exact resume boundary"))?;
        if eof {
            self.install_final_line_count(available)?;
        }
        self.prefix = Some(prefix::Input {
            resume_site,
            available,
            eof,
        });
        self.run_with_window(entry, PHASE_PREFIX, window)?;
        if eof
            && self
                .state
                .as_ref()
                .is_some_and(|state| state.line_matrices.captured.is_none())
        {
            self.resume_capture_tail(entry)?;
        }
        Ok(())
    }

    #[cfg(any(
        feature = "canonical-native-program-host-runtime",
        all(test, feature = "canonical-native-program")
    ))]
    fn resume_capture_tail<F: FnMut(*mut Frame) -> u64>(
        &mut self,
        entry: &mut F,
    ) -> Result<(), NativeProgramExecutionError> {
        let exit = self
            .descriptor
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("capture tail has no scan proof"))?
            .tail
            .exit();
        let input = self
            .prefix
            .as_mut()
            .ok_or_else(|| invalid("capture tail lost EOF"))?;
        if !input.eof || self.paused != Some(exit) {
            return Err(invalid("capture tail before exact scanner exit"));
        }
        input.resume_site = exit;
        self.run_with_window(entry, PHASE_CAPTURE_TAIL, None)
    }

    fn run<F: FnMut(*mut Frame) -> u64>(
        &mut self,
        entry: &mut F,
        phase: u64,
    ) -> Result<(), NativeProgramExecutionError> {
        self.run_with_window(entry, phase, None)
    }

    fn run_with_window<F: FnMut(*mut Frame) -> u64>(
        &mut self,
        entry: &mut F,
        phase: u64,
        line_window: Option<&mech_engine::native_artifact::NativeTextLineWindow<'_>>,
    ) -> Result<(), NativeProgramExecutionError> {
        if phase == PHASE_CAPTURE_TAIL && line_window.is_some() {
            return Err(invalid("capture tail cannot retain a scanner row loan"));
        }
        let descriptor = self.descriptor;
        let plan = &descriptor.plan;
        let boundary = plan
            .activation_boundary()
            .ok_or_else(|| invalid("missing certified activation boundary"))?;
        let prefix_input = if phase == PHASE_PREFIX || phase == PHASE_CAPTURE_TAIL {
            let input = self
                .prefix
                .ok_or_else(|| invalid("prefix entry without owned continuation facts"))?;
            if input.resume_site != self.paused.unwrap_or(boundary)
                || !self.activated
                || descriptor.prefix.is_none()
            {
                return Err(invalid("prefix entry outside owned resume boundary"));
            }
            Some(input)
        } else {
            if self.prefix.is_some() || self.paused.is_some() {
                return Err(invalid("ordinary entry cannot replay a prefix invocation"));
            }
            None
        };
        let mut state = self
            .state
            .take()
            .ok_or_else(|| invalid("consumed native state"))?;
        if let Some(window) = line_window {
            prefix::validate_window(descriptor, prefix_input, window, &state.library)?;
            let spent = window.producer_compute_work();
            if spent < self.window_work_seen {
                return Err(invalid("line window work regressed"));
            }
            // NativeWorkMeter already owns this SAME shared library account.
            // Framing/row-loan debits are included immediately, not billed twice.
            self.window_work_seen = spent;
        } else if self.window_work_seen != 0 && phase != PHASE_CAPTURE_TAIL {
            return Err(invalid("line window continuation lost its owning source"));
        }
        let checkpoint_work = if prefix_input.is_some() {
            amount(prefix::checkpoint_words(descriptor)?, 16, 256)?
        } else {
            0
        };
        // Both entry prologues and checkpoint verification are paid on this
        // SAME ledger before entering native code or scanning progress arrays.
        let transition_work = amount(plan.iterations().len(), 32, 512)?
            .checked_add(amount(plan.collections().len(), 32, 0)?)
            .and_then(|work| work.checked_add(checkpoint_work))
            .and_then(|work| {
                work.checked_add(if prefix_input.is_some() {
                    descriptor
                        .prefix
                        .as_ref()
                        .map_or(0, |region| region.header_work)
                } else {
                    0
                })
            })
            .ok_or_else(|| invalid("native phase work overflow"))?;
        let transitioned = state
            .work
            .charge(NativeExecutionPhase::Setup, transition_work);
        #[cfg(all(
            test,
            feature = "canonical-native-program",
            feature = "canonical-native-program-host-runtime"
        ))]
        setup_work_diagnostic::checkpoint(state.work.used());
        transitioned?;
        let (input_words, rest) = self
            .storage
            .values_mut()
            .split_at_mut(plan.transport_input_count());
        let (values, rest) = rest.split_at_mut(self.locals);
        let (progress_words, rest) = rest.split_at_mut(self.progress);
        let (output_words, rest) = rest.split_at_mut(plan.transport_output_count());
        let (result, rest) = rest.split_at_mut(1);
        let (arguments, retirements) = rest.split_at_mut(descriptor.max_library_inputs.max(1));
        let input_pointer = input_words.as_ptr();
        let local_pointer = values.as_mut_ptr();
        let output_pointer = output_words.as_mut_ptr();
        let result_pointer = result.as_mut_ptr();
        let argument_pointer = arguments.as_mut_ptr();
        let mut context = Context {
            // Native frame pointers/capacities name only the earlier lanes.
            // This immutable final lane is private to the checked callback.
            retirements,
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
            line_matrices: state.line_matrices,
            arena: state.arena,
            library: state.library,
            limits: self.limits.library,
            result: result_pointer,
            arguments: argument_pointer,
            argument_capacity: descriptor.max_library_inputs,
            call_inputs: state.call_inputs,
            call_caches: state.call_caches,
            consumed: state.consumed,
            output_cursor: state.output_cursor,
            error: None,
            activation_phase: phase == PHASE_ACTIVATION,
            window_region: None,
            borrowed_strings: None,
            closed_tail: None,
            selected_join: None,
            prefix: prefix_input.map(|input| prefix::State::new(input, self.retention)),
            line_window,
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
            resume_site: prefix_input.map_or(0, |input| u64::from(input.resume_site)),
            available_lines: prefix_input.map_or(0, |input| input.available),
            input_eof: prefix_input.map_or(0, |input| u64::from(input.eof)),
        };
        let status = entry(&mut frame);
        #[cfg(all(
            test,
            feature = "canonical-native-program",
            feature = "canonical-native-program-host-runtime"
        ))]
        setup_work_diagnostic::checkpoint(context.control.work.used());
        if let Some(error) = context.error.take() {
            return Err(error);
        }
        if context.window_region.is_some()
            || context.borrowed_strings.is_some()
            || context.closed_tail.is_some()
            || context.selected_join.is_some()
        {
            return Err(invalid("native entry returned with compiled String loans"));
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
            || frame.resume_site != prefix_input.map_or(0, |input| u64::from(input.resume_site))
            || frame.available_lines != prefix_input.map_or(0, |input| input.available)
            || frame.input_eof != prefix_input.map_or(0, |input| u64::from(input.eof))
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
                STATUS_OK if phase == PHASE_TURN || phase == PHASE_CAPTURE_TAIL => {
                    if !context.control.completed
                        || f.error_node != NO_NODE
                        || context.output_cursor != plan.transport_output_count()
                        || context.control.previous.is_some_and(|site| {
                            context.action_required(site as usize) && !context.consumed
                        })
                    {
                        return Err(invalid("program omitted atomic completion"));
                    }
                    self.paused = None;
                }
                STATUS_INPUT_PENDING if phase == PHASE_PREFIX => {
                    let site = context
                        .prefix
                        .as_ref()
                        .and_then(|prefix| prefix.paused)
                        .ok_or_else(|| invalid("native entry forged an input checkpoint"))?;
                    if f.error_node != u64::from(site) || prefix_input.is_none_or(|input| input.eof)
                    {
                        return Err(invalid("native entry returned the wrong input checkpoint"));
                    }
                    // check_prefix_header deliberately rejects an already
                    // marked pause. Recheck its invariant without granting a visit.
                    context.prefix.as_mut().unwrap().paused = None;
                    context.check_prefix_header(site)?;
                    prefix::check_cleared(descriptor, values, arguments)?;
                    self.retention =
                        Some(context.prefix.as_ref().unwrap().retained.ok_or_else(|| {
                            invalid("native checkpoint omitted retention coordinates")
                        })?);
                    self.paused = Some(site);
                }
                STATUS_INPUT_SCANNED if phase == PHASE_PREFIX => {
                    let site = context
                        .prefix
                        .as_ref()
                        .and_then(|prefix| prefix.scanned)
                        .ok_or_else(|| invalid("native entry forged scanner completion"))?;
                    if f.error_node != u64::from(site)
                        || prefix_input.is_none_or(|input| !input.eof)
                    {
                        return Err(invalid("native scanner returned wrong EOF checkpoint"));
                    }
                    context.prefix.as_mut().unwrap().scanned = None;
                    context.check_prefix_exit(site)?;
                    prefix::check_cleared(descriptor, values, arguments)?;
                    self.paused = Some(site);
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
            line_matrices: context.line_matrices,
            arena: context.arena,
            call_inputs: context.call_inputs,
            call_caches: context.call_caches,
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
    capture_offered: bool,
    capture: Option<capture_host::Capture>,
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
        let capture_offered = admission.text_capture_site().is_some();
        let mut prepared = Prepared::new(descriptor, limits, memory)?;
        prepared.activate(&mut |frame| entry.invoke(frame))?;
        Ok(Self {
            prepared,
            entry,
            invocation,
            capture_offered,
            capture: None,
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
        if self.capture.is_some() {
            return self.finish_text_capture(turn, inputs, outputs);
        } else if turn.text_capture().is_some() {
            return Err(invalid("native snapshot session received scoped capture"));
        }
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
