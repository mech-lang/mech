//! Owning two-phase state for ONE generated CFG invocation. Raw frame pointers
//! are reconstructed from admitted owned backing for each synchronous entry;
//! neither caller pointers nor self-referential borrowed state are retained.
use crate::native_artifact::abi::STATUS_INTEGRITY;
use crate::native_artifact::program::protocol::abi::{
    PHASE_ACTIVATION, PHASE_CAPTURE_TAIL, PHASE_PREFIX, PHASE_TURN, PHASE_WORKER, STATUS_ACTIVATED,
    STATUS_INPUT_PENDING, STATUS_INPUT_SCANNED, STATUS_WORKER_RESULT,
};
use crate::native_artifact::program::runtime::*;

mod call_cache_allocation;

mod capture;
#[cfg(feature = "canonical-native-program-host-runtime")]
mod capture_admission;
#[cfg(feature = "canonical-native-program-host-runtime")]
mod capture_host;

#[cfg(test)]
mod observation_tests;

#[cfg(all(test, feature = "canonical-native-program"))]
mod call_cache_tests;
#[cfg(all(test, feature = "canonical-native-program-host"))]
mod capture_host_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
mod capture_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
mod identity_inventory_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
mod prefix_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
mod prefix_window_tests;

#[cfg(all(test, feature = "canonical-native-program"))]
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
    let plan = &descriptor.metadata().plan;
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
pub(in crate::native_artifact::program::runtime) enum CallCacheEntry {
    Once,
    Repeated(NativePreparedLibraryCallCache, u64),
}

/// Empty prepared-call slots are invocation metadata, not prebound calls.
/// Their complete concrete backing is admitted during setup. Once slots have
/// no cache owner; repeated slots separately admit lazy bound metadata.
pub(in crate::native_artifact::program::runtime) struct CallCacheLane<'plan> {
    layout: &'plan crate::native_artifact::program::prepare::call_cache::Layout,
    entries: Box<[CallCacheEntry]>,
    metadata_nodes: u64,
    metadata_valid: bool,
    library: Option<NativeLibraryWorkBudget>,
    _reservation: mech_core::ManagedMemoryReservation,
    _nodes: mech_core::ManagedNodeReservation,
}
impl<'plan> CallCacheLane<'plan> {
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn entry(
        &self,
        site: usize,
    ) -> Option<&CallCacheEntry> {
        self.entries.get(self.layout.slot(site)?)
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(in crate::native_artifact::program::runtime) fn test_entries(&self) -> &[CallCacheEntry] {
        &self.entries
    }
    /// Reconcile only the cache that can change at this boundary. A failed
    /// reconciliation leaves the lane unusable, never a stale lower bound.
    pub(in crate::native_artifact::program::runtime) fn with_repeated<T>(
        &mut self,
        site: usize,
        work: &NativeLibraryWorkBudget,
        invoke: impl FnOnce(
            &mut NativePreparedLibraryCallCache,
        ) -> Result<T, NativeCollectionInvocationError>,
    ) -> Result<T, NativeCollectionInvocationError> {
        self.check_account(work)?;
        work.charge(256, 0)?;
        let slot = self
            .layout
            .slot(site)
            .ok_or(NativeCollectionInvocationError::InvalidInput)?;
        let CallCacheEntry::Repeated(cache, nodes) = self
            .entries
            .get_mut(slot)
            .ok_or(NativeCollectionInvocationError::InvalidInput)?
        else {
            return Err(NativeCollectionInvocationError::InvalidInput);
        };
        if self.library.is_none() {
            self.library = Some(work.clone());
        }
        self.metadata_valid = false;
        let outcome = invoke(cache);
        // A failed call may still have populated admitted plans or indexes.
        // Reconcile those holdings before returning either result.
        let next = cache.retained_metadata_demand(work)?.retained_nodes;
        self.metadata_nodes = self
            .metadata_nodes
            .checked_sub(*nodes)
            .and_then(|total| total.checked_add(next))
            .ok_or(NativeCollectionInvocationError::InvalidInput)?;
        *nodes = next;
        self.metadata_valid = true;
        outcome
    }

    pub(in crate::native_artifact::program::runtime) fn invoke(
        &mut self,
        site: usize,
        call: &mech_engine::native_artifact::NativePreparedLibraryCall,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, NativeCollectionInvocationError> {
        match self.entry(site) {
            Some(CallCacheEntry::Once) => call.invoke_current(inputs, output, limits, work),
            Some(CallCacheEntry::Repeated(..)) => self.with_repeated(site, work, |cache| {
                call.invoke_current_cached(cache, inputs, output, limits, work)
            }),
            None => Err(NativeCollectionInvocationError::InvalidInput),
        }
    }

    fn check_account(
        &self,
        work: &NativeLibraryWorkBudget,
    ) -> Result<(), NativeCollectionInvocationError> {
        if !self.metadata_valid
            || self._reservation.budget() != work.memory_budget()
            || self
                .library
                .as_ref()
                .is_some_and(|owner| !owner.shares_account(work))
        {
            return Err(NativeCollectionInvocationError::UnadmittedValue);
        }
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime) fn caller_retained_nodes(
        &self,
        work: &NativeLibraryWorkBudget,
    ) -> Result<u64, NativeCollectionInvocationError> {
        self.check_account(work)?;
        work.charge(256, 0)?;
        self._reservation
            .capacity_bytes()
            .checked_add(self.metadata_nodes)
            .ok_or(NativeCollectionInvocationError::InvalidInput)
    }

    pub(in crate::native_artifact::program::runtime) fn metadata_bytes(
        count: usize,
    ) -> Result<u64, NativeProgramExecutionError> {
        amount(
            count,
            (core::mem::size_of::<CallCacheEntry>() as u64) * 2,
            128,
        )
    }
    fn new(
        descriptor: &'plan NativeProgramDescriptor,
        limit: u64,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Self, NativeProgramExecutionError> {
        let layout = &descriptor.metadata().call_cache;
        let count = layout.len();
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
        let nodes = memory.reserve_nodes(bytes)?;
        // PreparedNativeProgram::prepare already ran BOTH canonical phase
        // and current-visit proofs. check_scope contracts each child loop to
        // initializer -> exit and rejects every remaining scope cycle. Thus
        // None is a once-only root DAG site, including nested Match arms;
        // Some covers counted, collection, tail and enclosing nested loops.
        // Phase resume retains the same previous/current/progress state and
        // cannot replay a root block. No new execution authority is created.
        let entries = call_cache_allocation::allocate(count, bytes, |ordinal| {
            if !layout.repeated()[ordinal] {
                CallCacheEntry::Once
            } else {
                CallCacheEntry::Repeated(NativePreparedLibraryCallCache::default(), 0)
            }
        })?;
        Ok(Self {
            layout,
            entries: entries.into_boxed_slice(),
            metadata_nodes: 0,
            metadata_valid: true,
            library: None,
            _reservation: reservation,
            _nodes: nodes,
        })
    }
}

pub(in crate::native_artifact::program::runtime) struct Prepared<'a> {
    descriptor: &'a NativeProgramDescriptor,
    state: Option<ExecutionState<'a>>,
    storage: NativeWordStorage,
    memory: ManagedMemoryBudget,
    limits: NativeProgramExecutionLimits,
    locals: usize,
    progress: usize,
    bytes: u64,
    activated: bool,
    prefix: Option<crate::native_artifact::program::runtime::invocation::prefix::Input>,
    paused: Option<u32>,
    retention: Option<crate::native_artifact::program::runtime::invocation::prefix::Retention>,
    window_work_seen: u64,
    worker: Option<crate::native_artifact::program::runtime::invocation::parallel::Worker>,
    // Invocation/frame roots outlive every synchronous Invocation and worker.
    // Payload Values and private owners hold their separate shared claims.
    _fixed_nodes: mech_core::ManagedNodeReservation,
}

struct SetupGeometry {
    locals: usize,
    progress: usize,
    words: usize,
    bytes: u64,
}

impl<'a> Prepared<'a> {
    /// Exact fixed private node claims made by `new_with_budget`. This is a
    /// necessary simultaneous setup floor, not a promise that later canonical
    /// constants, cache entries, builders or map contents will fit. Those keep
    /// their existing atomic admission at construction/publication.
    pub(in crate::native_artifact::program::runtime) fn worker_setup_nodes(
        descriptor: &NativeProgramDescriptor,
    ) -> Result<u64, NativeProgramExecutionError> {
        let fixed =
            crate::native_artifact::program::runtime::invocation::caller_geometry::fixed_nodes()?;
        let collections =
            crate::native_artifact::program::runtime::collections::State::metadata_bytes(
                descriptor,
            )?;
        let call_caches = CallCacheLane::metadata_bytes(descriptor.metadata().call_cache.len())?;
        let line_matrices = u64::from(!descriptor.metadata().line_matrices.regions.is_empty());
        let frames = super::buffer_frames::BufferFrames::metadata_nodes(
            &descriptor.metadata().buffer_frames,
        )?;
        let total = fixed
            .checked_add(frames)
            .ok_or_else(|| invalid("worker frame node count overflow"))?
            .checked_add(collections)
            .and_then(|n| n.checked_add(call_caches))
            .and_then(|n| n.checked_add(line_matrices))
            .ok_or_else(|| invalid("worker fixed setup node count overflow"))?;
        Ok(total)
    }

    /// Fixed constructor admission envelope: transport backing and its core
    /// initialization metadata, private lanes, and the declared constant-import
    /// bounds. Later builders/cache entries keep independent checked admission.
    pub(in crate::native_artifact::program::runtime) fn worker_setup_bytes(
        descriptor: &NativeProgramDescriptor,
    ) -> Result<u64, NativeProgramExecutionError> {
        let geometry = Self::setup_geometry(descriptor)?;
        let mut bytes = geometry.bytes;
        for (raw, admitted) in [
            (
                transport_bytes(geometry.words)?,
                NativeWordStorage::admission_bytes(geometry.words)?,
            ),
            (
                amount(
                    descriptor.metadata().types.len(),
                    size_of::<Option<Value>>() as u64,
                    0,
                )?,
                NativeTypedStorage::<Option<Value>>::admission_bytes(
                    descriptor.metadata().types.len(),
                )?,
            ),
            (
                transport_bytes(descriptor.metadata().types.len())?,
                NativeWordStorage::admission_bytes(descriptor.metadata().types.len())?,
            ),
            (
                amount(
                    descriptor.metadata().max_library_inputs,
                    size_of::<Option<Value>>() as u64,
                    0,
                )?,
                NativeTypedStorage::<Option<Value>>::admission_bytes(
                    descriptor.metadata().max_library_inputs,
                )?,
            ),
        ] {
            bytes = bytes
                .checked_sub(raw)
                .and_then(|bytes| bytes.checked_add(admitted))
                .ok_or_else(|| invalid("worker fixed backing admission overflow"))?;
        }
        let frames = &descriptor.metadata().buffer_frames;
        if frames.elements() != 0 {
            bytes = bytes
                .checked_sub(super::buffer_frames::BufferFrames::raw_bytes(frames)?)
                .and_then(|n| {
                    n.checked_add(super::buffer_frames::BufferFrames::admission_bytes(frames).ok()?)
                })
                .ok_or_else(|| invalid("private frame admission overflow"))?;
        }
        // This immutable sum is established at prepare/decode time, so a
        // repeated dispatch never performs an unfunded constants-table scan.
        bytes
            .checked_add(descriptor.metadata().constant_admission_bytes)
            .ok_or_else(|| invalid("worker constant admission overflow"))
    }

    fn setup_geometry(
        descriptor: &NativeProgramDescriptor,
    ) -> Result<SetupGeometry, NativeProgramExecutionError> {
        let plan = &descriptor.metadata().plan;
        let locals = descriptor.metadata().local_words().map_err(invalid)?;
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
            .and_then(|n| n.checked_add(descriptor.metadata().max_library_inputs.max(1)))
            .and_then(|n| n.checked_add(descriptor.metadata().retirements.words().len()))
            .ok_or_else(|| invalid("transport count overflow"))?;
        let word_and_frame_bytes = transport_bytes(words)?
            .checked_add(super::buffer_frames::BufferFrames::raw_bytes(
                &descriptor.metadata().buffer_frames,
            )?)
            .ok_or_else(|| invalid("private frame transport overflow"))?;
        let word_and_frame_bytes = word_and_frame_bytes
            .checked_add(super::buffer_frames::BufferFrames::metadata_bytes(
                &descriptor.metadata().buffer_frames,
            ))
            .ok_or_else(|| invalid("private frame metadata overflow"))?;
        let bytes = word_and_frame_bytes
            .checked_add(amount(
                descriptor.metadata().types.len(),
                (core::mem::size_of::<Option<Value>>() + 8) as u64,
                0,
            )?)
            .and_then(|n| {
                n.checked_add(
                    amount(
                        descriptor.metadata().max_library_inputs,
                        core::mem::size_of::<Option<Value>>() as u64,
                        0,
                    )
                    .ok()?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    crate::native_artifact::program::runtime::collections::State::metadata_bytes(
                        descriptor,
                    )
                    .ok()?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    CallCacheLane::metadata_bytes(descriptor.metadata().call_cache.len()).ok()?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    crate::native_artifact::program::runtime::lines::State::metadata_bytes(
                        descriptor.metadata().line_matrices.regions.len(),
                    )
                    .ok()?,
                )
            })
            .ok_or_else(|| invalid("combined transport overflow"))?;
        Ok(SetupGeometry {
            locals,
            progress,
            words,
            bytes,
        })
    }

    pub(in crate::native_artifact::program::runtime) fn new(
        descriptor: &'a NativeProgramDescriptor,
        limits: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
    ) -> Result<Self, NativeProgramExecutionError> {
        Self::new_with_budget(descriptor, limits, memory, None)
    }

    fn new_with_budget(
        descriptor: &'a NativeProgramDescriptor,
        limits: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
        worker_budget: Option<NativeLibraryWorkBudget>,
    ) -> Result<Self, NativeProgramExecutionError> {
        let plan = &descriptor.metadata().plan;
        if plan.activation_boundary().is_none() {
            return Err(invalid(
                "managed execution has no certified activation boundary",
            ));
        }
        if descriptor.stack_bytes() > limits.execution.max_native_stack_bytes {
            return Err(NativeExecutionError::StackLimit {
                required: descriptor.stack_bytes(),
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
        let SetupGeometry {
            locals,
            progress,
            words,
            bytes,
        } = Self::setup_geometry(descriptor)?;
        if bytes > limits.execution.max_transport_bytes {
            return Err(NativeExecutionError::TransportLimit {
                required: bytes,
                limit: limits.execution.max_transport_bytes,
            }
            .into());
        }
        let mut work = if let Some(library) = &worker_budget {
            NativeWorkMeter::from_worker_budget(library.clone(), memory)?
        } else {
            NativeWorkMeter::new(limits.execution.max_work)?
        };
        work.charge(NativeExecutionPhase::Setup, amount(words, 4, 256)?)?;
        work.charge(
            NativeExecutionPhase::Setup,
            amount(
                descriptor.metadata().max_library_inputs,
                (core::mem::size_of::<Option<Value>>() as u64) * 8 + 64,
                4096,
            )?,
        )?;
        let library = if let Some(library) = worker_budget {
            library
        } else {
            work.shared_library_budget(
                NativeExecutionPhase::Setup,
                caps.max_comparison_work.unwrap(),
                memory,
            )?
        };
        let fixed_nodes = memory.reserve_nodes(
            crate::native_artifact::program::runtime::invocation::caller_geometry::fixed_nodes()?,
        )?;
        let mut storage =
            NativeWordStorage::new(words, limits.execution.max_transport_bytes, Some(memory))?;
        // The exact table occupies the disjoint final lane of this SAME
        // admitted typed arena; no new runtime plan or unowned metadata pool.
        // Initialization was prepaid above; fund its copy before touching it.
        work.charge(
            NativeExecutionPhase::Setup,
            amount(descriptor.metadata().retirements.words().len(), 16, 128)?,
        )?;
        storage.values_mut()[words - descriptor.metadata().retirements.words().len()..]
            .copy_from_slice(descriptor.metadata().retirements.words());
        let mut arena = NativeValueArena::new(
            &descriptor.metadata().types,
            limits.execution.max_transport_bytes,
            memory,
            &mut work,
        )?;
        let buffer_frames = super::buffer_frames::BufferFrames::new(
            &descriptor.metadata().buffer_frames,
            limits.execution.max_transport_bytes,
            memory,
            &mut work,
        )?;
        let call_inputs = NativeTypedStorage::new_values(
            descriptor.metadata().max_library_inputs,
            limits.execution.max_transport_bytes,
            memory,
        )?;
        // Constants and their retirement are funded only once, before any
        // generated activation. No fresh Values or observations are touched.
        for (constant, &bound) in plan
            .constants()
            .iter()
            .zip(&descriptor.metadata().constant_bytes)
        {
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
        let collections = crate::native_artifact::program::runtime::collections::State::new(
            descriptor, caps, memory, &mut work,
        )?;
        let call_caches = CallCacheLane::new(
            descriptor,
            limits.execution.max_transport_bytes,
            memory,
            &mut work,
        )?;
        let line_matrices = crate::native_artifact::program::runtime::lines::State::new(
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
            worker: None,
            _fixed_nodes: fixed_nodes,
            state: Some(ExecutionState {
                map_owners: crate::native_artifact::program::runtime::maps::owner::Session::new(
                    descriptor,
                ),
                collections,
                line_matrices,
                arena,
                call_inputs,
                buffer_frames,
                call_caches,
                library,
                control: ControlState::new(work),
                stats: RuntimeStats::default(),
                consumed: false,
                scalar_path: None,
                output_cursor: 0,
            }),
        })
    }

    pub(in crate::native_artifact::program::runtime) fn run_worker(
        descriptor: &'a NativeProgramDescriptor,
        entry: crate::native_artifact::program::protocol::abi::Entry,
        limits: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
        library: NativeLibraryWorkBudget,
        worker: crate::native_artifact::program::runtime::invocation::parallel::Worker,
        captures: Vec<crate::native_artifact::program::runtime::invocation::parallel::Capture>,
    ) -> Result<
        crate::native_artifact::program::runtime::invocation::parallel::Receipt,
        NativeProgramExecutionError,
    > {
        let region = descriptor
            .metadata()
            .parallel
            .regions
            .get(worker.region)
            .ok_or_else(|| invalid("worker constructor has no same-plan proof"))?;
        if !crate::native_artifact::program::runtime::invocation::parallel::admission::supported(
            descriptor,
            worker.region,
        ) {
            return Err(invalid(
                "worker constructor lacks shared private-node admission",
            ));
        }
        if worker.index >= worker.count
            || worker.count == 0
            || worker.result.is_some()
            || captures.len() != region.captures.len()
        {
            return Err(invalid("worker constructor capture/count mismatch"));
        }
        let mut prepared = Self::new_with_budget(descriptor, limits, memory, Some(library))?;
        let locals_start = descriptor.metadata().plan.transport_input_count();
        let state = prepared.state.as_mut().unwrap();
        // A worker skips whole-program activation, but retains the SAME
        // immutable constants. Its private local lanes receive their original
        // words; no source computation or observation is rerun here.
        for constant in descriptor.metadata().plan.constants() {
            if let Some(kind) =
                descriptor.metadata().plan.values()[constant.value().get() as usize].scalar_kind()
            {
                prepared.storage.values_mut()[locals_start + constant.value().get() as usize] =
                    bits(constant.constant(), kind)?;
            }
        }
        for (target, words) in descriptor.metadata().scalar_aggregates.constants() {
            let (start, count) = descriptor
                .metadata()
                .scalar_aggregates
                .slot(*target)
                .ok_or_else(|| invalid("worker aggregate constant has no words"))?;
            if count != words.len() {
                return Err(invalid("worker aggregate constant shape"));
            }
            prepared.storage.values_mut()[locals_start + start..locals_start + start + count]
                .copy_from_slice(words);
        }
        for (&expected, capture) in region.captures.iter().zip(captures) {
            match capture {
                crate::native_artifact::program::runtime::invocation::parallel::Capture::Word(
                    slot,
                    word,
                ) if slot == expected => {
                    let kind = descriptor.metadata().plan.values()[slot as usize]
                        .scalar_kind()
                        .ok_or_else(|| invalid("worker scalar capture type"))?;
                    if !kind.accepts_bits(word) {
                        return Err(invalid("worker scalar capture exceeds its type"));
                    }
                    prepared.storage.values_mut()[locals_start + slot as usize] = word;
                }
                crate::native_artifact::program::runtime::invocation::parallel::Capture::Value(
                    slot,
                    value,
                ) if slot == expected => {
                    if descriptor.metadata().plan.values()[slot as usize]
                        .scalar_kind()
                        .is_some()
                    {
                        return Err(invalid("worker managed capture type"));
                    }
                    state.arena.put(slot, &value, &mut state.control.work)?;
                }
                _ => return Err(invalid("worker capture inventory differs")),
            }
        }
        prepared.worker = Some(worker);
        prepared.activated = true;
        prepared.run(
            &mut |frame| {
                // SAFETY: the scoped parent retains this exact checked code owner;
                // this constructor owns all of the child's disjoint frame lanes.
                unsafe { entry(frame) }
            },
            PHASE_WORKER,
        )?;
        let receipt = prepared
            .worker
            .as_mut()
            .and_then(|worker| worker.result.take())
            .ok_or_else(|| invalid("worker omitted its owned result"))?;
        // Retire every Rc work account before returning across the thread join.
        drop(prepared);
        Ok(receipt)
    }

    pub(in crate::native_artifact::program::runtime) fn activate<F: FnMut(*mut Frame) -> u64>(
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
        let plan = &descriptor.metadata().plan;
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
            state.arena.put(
                plan.inputs()[index].value().get(),
                value,
                &mut state.control.work,
            )?;
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
                &descriptor.metadata().schemas,
                descriptor.metadata().observation_schema_bytes,
                &self.memory,
                &mut state.control.work,
            )?;
            state.arena.put(
                plan.transport_input_value(index).get(),
                &owned,
                &mut state.control.work,
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

    pub(in crate::native_artifact::program::runtime) fn finish<F: FnMut(*mut Frame) -> u64>(
        mut self,
        entry: &mut F,
        inputs: &[Value],
        observations: &[Value],
        outputs: &mut [Option<Value>],
    ) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
        let imported = self.import_turn(inputs, observations, outputs);
        imported?;
        self.run(entry, PHASE_TURN)?;
        self.publish(outputs)
    }

    fn publish(
        mut self,
        outputs: &mut [Option<Value>],
    ) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
        if self.paused.is_some()
            || !self
                .state
                .as_ref()
                .is_some_and(|state| state.control.completed)
        {
            return Err(invalid("publication before exact native completion"));
        }
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        let published = state.arena.publish_range(
            self.descriptor.metadata().plan.values().len(),
            outputs,
            &mut state.control.work,
        );
        published?;
        Ok(NativeProgramExecutionReport {
            work: state.control.work.used(),
            comparison_work: state.library.used().1,
            visits: state.control.visits,
            transport_bytes: self.bytes,
            hash_table_regions: state.stats.hash_table_regions,
            hash_table_updates: state.stats.hash_table_updates,
            packed_map_regions: state.stats.packed_map_regions,
            direct_lookup_regions: state.stats.direct_lookup_regions,
            packed_map_updates: state.stats.packed_map_updates,
            packed_ssa_regions: state.stats.packed_ssa_regions,
            guarded_loop_regions: state.stats.guarded_loop_regions,
            guarded_call_regions: state.stats.guarded_call_regions,
            frozen_map_regions: state.stats.frozen_map_regions,
            frozen_map_reads: state.stats.frozen_map_reads,
            worker_credit_claims: state.stats.worker_credit_claims,
            worker_credit_batches: state.stats.worker_credit_batches,
            worker_credit_reclaims: state.stats.worker_credit_reclaims,
            fused_string_map_bytes: state.stats.fused_string_map_bytes,
            parallel_regions: state.stats.parallel_regions,
            parallel_batches: state.stats.parallel_batches,
            parallel_tasks: state.stats.parallel_tasks,
            parallel_peak_workers: state.stats.parallel_peak_workers,
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
            || self.descriptor.metadata().prefix.is_none()
            || self
                .state
                .as_ref()
                .is_none_or(|state| state.control.completed)
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
                crate::native_artifact::program::runtime::invocation::prefix::validate_window(
                    self.descriptor,
                    Some(
                        crate::native_artifact::program::runtime::invocation::prefix::Input {
                            resume_site: self
                                .descriptor
                                .metadata()
                                .plan
                                .activation_boundary()
                                .ok_or_else(|| invalid("prefix lacks exact resume boundary"))?,
                            available,
                            eof,
                        },
                    ),
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
            .or(self.descriptor.metadata().plan.activation_boundary())
            .ok_or_else(|| invalid("prefix lacks exact resume boundary"))?;
        if eof {
            self.install_final_line_count(available)?;
        }
        self.prefix = Some(
            crate::native_artifact::program::runtime::invocation::prefix::Input {
                resume_site,
                available,
                eof,
            },
        );
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
            .metadata()
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
        if (phase == PHASE_WORKER) != self.worker.is_some() {
            return Err(invalid("worker phase requires constructor-owned state"));
        }
        if phase == PHASE_CAPTURE_TAIL && line_window.is_some() {
            return Err(invalid("capture tail cannot retain a scanner row loan"));
        }
        let descriptor = self.descriptor;
        let plan = &descriptor.metadata().plan;
        let boundary = plan
            .activation_boundary()
            .ok_or_else(|| invalid("missing certified activation boundary"))?;
        let prefix_input = if phase == PHASE_PREFIX || phase == PHASE_CAPTURE_TAIL {
            let input = self
                .prefix
                .ok_or_else(|| invalid("prefix entry without owned continuation facts"))?;
            if input.resume_site != self.paused.unwrap_or(boundary)
                || !self.activated
                || descriptor.metadata().prefix.is_none()
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
            crate::native_artifact::program::runtime::invocation::prefix::validate_window(
                descriptor,
                prefix_input,
                window,
                &state.library,
            )?;
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
            amount(
                crate::native_artifact::program::runtime::invocation::prefix::checkpoint_words(
                    descriptor,
                )?,
                16,
                256,
            )?
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
                        .metadata()
                        .prefix
                        .as_ref()
                        .map_or(0, |region| region.header_work)
                } else {
                    0
                })
            })
            .ok_or_else(|| invalid("native phase work overflow"))?;
        let transitioned = state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, transition_work);
        transitioned?;
        let (input_words, rest) = self
            .storage
            .values_mut()
            .split_at_mut(plan.transport_input_count());
        let (values, rest) = rest.split_at_mut(self.locals);
        let (progress_words, rest) = rest.split_at_mut(self.progress);
        let (output_words, rest) = rest.split_at_mut(plan.transport_output_count());
        let (result, rest) = rest.split_at_mut(1);
        let (arguments, retirements) =
            rest.split_at_mut(descriptor.metadata().max_library_inputs.max(1));
        let input_pointer = input_words.as_ptr();
        let local_pointer = values.as_mut_ptr();
        let output_pointer = output_words.as_mut_ptr();
        let result_pointer = result.as_mut_ptr();
        let argument_pointer = arguments.as_mut_ptr();
        let mut context = Invocation {
            // Native frame pointers/capacities name only the earlier lanes.
            // This immutable final lane is private to the checked callback.
            retirements,
            descriptor,
            progress: progress_words,
            state: &mut state,

            limits: self.limits.library,
            result: result_pointer,
            arguments: argument_pointer,
            argument_capacity: descriptor.metadata().max_library_inputs,
            error: None,
            activation_phase: phase == PHASE_ACTIVATION,
            execution_limits: self.limits.execution,
            worker: self.worker.take(),
            parallel_batch: None,
            window_region: None,
            window_hashes: None,
            borrowed_strings: None,
            closed_tail: None,
            parent_continuation: Default::default(),
            selected_join: None,
            prefix: prefix_input.map(|input| {
                crate::native_artifact::program::runtime::invocation::prefix::State::new(
                    input,
                    self.retention,
                )
            }),
            line_window,
        };
        let context_pointer = (&mut context as *mut Invocation<'_, '_, '_>).cast::<c_void>();
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
            argument_capacity: descriptor.metadata().max_library_inputs as u64,
            phase,
            resume_site: context.worker.as_ref().map_or_else(
                || prefix_input.map_or(0, |input| u64::from(input.resume_site)),
                |worker| u64::from(descriptor.metadata().parallel.regions[worker.region].begin),
            ),
            available_lines: context.worker.as_ref().map_or_else(
                || prefix_input.map_or(0, |input| input.available),
                |worker| worker.index,
            ),
            input_eof: context.worker.as_ref().map_or_else(
                || prefix_input.map_or(0, |input| u64::from(input.eof)),
                |worker| worker.count,
            ),
        };
        let phase_words = (frame.resume_site, frame.available_lines, frame.input_eof);
        let status = entry(&mut frame);
        if let Some(error) = context.error.take() {
            return Err(error);
        }
        if context.window_hashes.is_some()
            || context.window_region.is_some()
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
            || frame.argument_capacity != descriptor.metadata().max_library_inputs as u64
            || frame.phase != phase
            || (frame.resume_site, frame.available_lines, frame.input_eof) != phase_words
        {
            return Err(invalid("program entry changed immutable frame"));
        }
        if phase == PHASE_ACTIVATION && status == STATUS_ACTIVATED {
            let previous = context.admission_predecessor(u64::from(boundary))?;
            if f.error_node != u64::from(boundary)
                || context.state.control.completed
                || context.state.control.current.is_some()
                || context.progress.chunks_exact(2).any(|value| value[1] != 0)
                || context.state.output_cursor != 0
                || context
                    .state
                    .collections
                    .as_ref()
                    .is_some_and(|state| !state.is_complete())
                || previous.is_none_or(|previous| {
                    !descriptor.metadata().visits[previous as usize]
                        .successors()
                        .contains(&Some(boundary))
                        || (context.action_required(previous as usize) && !context.state.consumed)
                })
            {
                return Err(invalid("native entry omitted exact activation checkpoint"));
            }
        } else {
            match status {
                STATUS_WORKER_RESULT if phase == PHASE_WORKER => {
                    let worker = context
                        .worker
                        .as_ref()
                        .ok_or_else(|| invalid("worker state lost"))?;
                    let region = &descriptor.metadata().parallel.regions[worker.region];
                    if worker.result.is_none()
                        || f.error_node != u64::from(region.push)
                        || context.state.control.completed
                        || context.state.output_cursor != 0
                        || context.state.control.current != Some(region.iteration)
                        || context.parallel_batch.is_some()
                    {
                        return Err(invalid("worker omitted exact source result frontier"));
                    }
                }
                STATUS_OK if phase == PHASE_TURN || phase == PHASE_CAPTURE_TAIL => {
                    if !context.state.control.completed
                        || f.error_node != NO_NODE
                        || context.state.output_cursor != plan.transport_output_count()
                        || context.state.control.previous.is_some_and(|site| {
                            context.action_required(site as usize) && !context.state.consumed
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
                    crate::native_artifact::program::runtime::invocation::prefix::check_cleared(
                        descriptor, values, arguments,
                    )?;
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
                    crate::native_artifact::program::runtime::invocation::prefix::check_cleared(
                        descriptor, values, arguments,
                    )?;
                    self.paused = Some(site);
                }
                STATUS_ARITHMETIC | STATUS_REJECT | STATUS_INTEGRITY => {
                    let id = context.failure_site(f.error_node)?;
                    let site = &descriptor.metadata().visits[id as usize];
                    return Err(if status == STATUS_ARITHMETIC && site.arithmetic {
                        NativeIterationExecutionError::Arithmetic {
                            origin: site.origin,
                        }
                        .into()
                    } else if status == STATUS_INTEGRITY && site.integrity {
                        NativeIterationExecutionError::Integrity {
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
        if context.parallel_batch.is_some() {
            return Err(invalid("native phase left pending parallel results"));
        }
        self.worker = context.worker.take();
        drop(context);
        self.state = Some(state);
        Ok(())
    }
}

#[cfg(feature = "canonical-native-program-host-runtime")]
pub(in crate::native_artifact) enum CheckedEntry {
    #[cfg(feature = "canonical-native-program")]
    Jit(crate::native_artifact::program::protocol::abi::Entry),
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
    pub(in crate::native_artifact::program::runtime) fn prepare(
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
