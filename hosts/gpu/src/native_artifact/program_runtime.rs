//! Managed ABI16 invocation. This boundary never evaluates a graph: generated
//! blocks choose every successor and run every scalar/counter instruction.
use super::abi::{
    NO_NODE, NativeEntryFrame, STATUS_ABI, STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK,
};
use super::iteration_abi::STATUS_REJECT;
use super::iteration_runtime::{Context as Control, NativeIterationExecutionError};
use super::memory::{NativeTypedStorage, NativeWordStorage, transport_bytes};
use super::program_abi::{COLLECTION, Frame, LIBRARY, MOVE, OUTPUT_BASE, VERSION};
use super::program_descriptor::RetirementKind;
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
    NativeMatchValue, NativePreparedLibraryCallCache, NativeScalarKind,
};
use std::ffi::c_void;

#[path = "program_borrowed_strings.rs"]
mod borrowed;
#[path = "program_tail_runtime.rs"]
mod closed_tail;
#[path = "program_collection.rs"]
mod collection;
#[path = "program_line_matrices.rs"]
mod line_matrices;
#[path = "program_prefix_runtime.rs"]
mod prefix;
#[path = "program_selected_join.rs"]
mod selected_join;
#[path = "program_session.rs"]
mod session;
#[path = "program_parallel_runtime.rs"]
mod parallel;
#[path = "program_parallel_admission.rs"]
mod parallel_admission;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_parallel_tests.rs"]
mod parallel_tests;
#[path = "program_window_round.rs"]
mod window_round;
#[path = "program_caller_geometry.rs"]
mod caller_geometry;
#[path = "program_map_owner_runtime.rs"]
mod map_owners;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_map_admission_tests.rs"]
mod map_admission_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_map_closed_tests.rs"]
mod map_closed_tests;
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
    pub hash_table_regions: u64,
    pub hash_table_updates: u64,
    /// Regions that actually selected packed storage at runtime.
    pub packed_map_regions: u64,
    /// Updates completed by those selected packed regions.
    pub packed_map_updates: u64,
    /// Completed packed regions that entered the versioned SSA CFG.
    pub packed_ssa_regions: u64,
    /// Source comprehensions whose original child bodies completed in workers.
    pub parallel_regions: u64,
    /// Completed dispatches; one dispatch may cover several source regions.
    pub parallel_batches: u64,
    /// Original source elements completed by those worker batches.
    pub parallel_tasks: u64,
    /// Observed peak simultaneously executing workers, not admitted capacity.
    pub parallel_peak_workers: u64,
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
#[cfg(all(test, feature = "canonical-native-program"))]
fn test_corrupt_prefix_checkpoint(frame: *mut Frame, field: u8) {
    // SAFETY: only the synchronous owning Prepared test driver calls this,
    // after the retained checked JIT returned. Descriptor offsets name its
    // exact admitted lanes; no poisoned word is subsequently executed.
    unsafe {
        let frame = &mut *frame;
        assert_eq!(frame.phase, super::program_abi::PHASE_PREFIX);
        let context = &*frame.base.context.cast::<Context<'_, '_>>();
        let descriptor = context.control.descriptor;
        let region = descriptor.prefix.as_ref().unwrap();
        let borrowed = &descriptor.borrowed_strings.regions[region.borrowed];
        match field {
            0..=2 => {
                let word = match field {
                    0 => borrowed.ready,
                    1 => borrowed.end - 1,
                    _ => {
                        descriptor.closed_tails.regions
                            [descriptor.closed_tails.sites[region.tail.header() as usize].unwrap()]
                        .words
                            + 1
                    }
                };
                assert!(word < frame.base.local_count as usize);
                *frame.base.locals.add(word) = 1;
            }
            3 => {
                assert!(frame.argument_capacity > 0);
                *frame.arguments = 1;
            }
            4 => frame.resume_site ^= 1,
            5 => frame.available_lines ^= 1,
            6 => frame.input_eof ^= 1,
            _ => panic!("unknown checkpoint corruption"),
        }
    }
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
    retirements: &'visit [u64],
    control: Control<'visit, NativeProgramDescriptor>,
    // Sources are cheap shared roots from the arena; drop collection state
    // first, while that arena still owns their prepaid retirement obligation.
    map_owners: map_owners::Session<'plan>,
    collections: Option<collection::State>,
    borrowed_strings: Option<borrowed::State>,
    closed_tail: Option<closed_tail::State>,
    selected_join: Option<selected_join::State>,
    line_matrices: line_matrices::State,
    arena: NativeValueArena<'plan>,
    library: NativeLibraryWorkBudget,
    limits: MemoryBudgetLimits,
    result: *mut u64,
    arguments: *mut u64,
    argument_capacity: usize,
    call_inputs: NativeTypedStorage<Option<Value>>,
    call_caches: session::CallCacheLane,
    consumed: bool,
    output_cursor: usize,
    error: Option<NativeProgramExecutionError>,
    activation_phase: bool,
    execution_limits: NativeExecutionLimits,
    worker: Option<parallel::Worker>,
    parallel_batch: Option<parallel::Batch>,
    hash_table_regions: u64,
    hash_table_updates: u64,
    packed_map_regions: u64,
    packed_map_updates: u64,
    packed_ssa_regions: u64,
    parallel_regions: u64,
    parallel_batches: u64,
    parallel_tasks: u64,
    parallel_peak_workers: u64,
    window_region: Option<(u32, u64)>,
    window_hashes: Option<Box<window_round::hashes::WindowHashes>>,
    prefix: Option<prefix::State>,
    // The borrow pins real UTF-8 backing for this synchronous entry only.
    line_window: Option<&'visit mech_engine::native_artifact::NativeTextLineWindow<'visit>>,
}
impl Context<'_, '_> {
    fn write_borrowed_word(
        &mut self,
        index: usize,
        word: u64,
    ) -> Result<(), NativeProgramExecutionError> {
        if self.arguments.is_null() || index >= self.argument_capacity {
            return Err(invalid("borrowed String capture lane is too small"));
        }
        // SAFETY: the owning session registers this exact admitted lane.
        // Generated code is suspended; no reference survives this callback.
        unsafe {
            *self.arguments.add(index) = word;
        }
        Ok(())
    }
    fn write_window_arguments(
        &mut self,
        words: [u64; 5],
    ) -> Result<(), NativeProgramExecutionError> {
        if self.arguments.is_null() || self.argument_capacity < words.len() {
            return Err(invalid("compiled region argument lane is too small"));
        }
        // SAFETY: the owning session registers this exact disjoint five-word
        // lane. Generated code is suspended in this callback, and the loaned
        // canonical owners in call_inputs outlive its subsequent native read.
        unsafe { core::slice::from_raw_parts_mut(self.arguments, words.len()) }
            .copy_from_slice(&words);
        Ok(())
    }
    fn needs_move(&self, site: usize) -> Option<(NativeMatchValue, NativeMatchValue)> {
        if self.borrowed_action(site).is_some() || self.retained_native_action(site) {
            return None;
        }
        if self
            .control
            .descriptor
            .scalar_aggregates
            .action(site)
            .is_some()
        {
            return None;
        }
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
        if self.borrowed_action(site).is_some() || self.retained_native_action(site) {
            return false;
        }
        if self
            .control
            .descriptor
            .scalar_aggregates
            .action(site)
            .is_some()
        {
            return false;
        }
        matches!(
            self.control.descriptor.plan.blocks()[site].action(),
            NativeIterationAction::LibraryCall { .. }
                | NativeIterationAction::PrepareU64Range { .. }
                | NativeIterationAction::PrepareStringSpanSelector { .. }
                | NativeIterationAction::ProjectStringSpan { .. }
                | NativeIterationAction::ProjectStringMapEntry { .. }
                | NativeIterationAction::Structural { .. }
                | NativeIterationAction::CollectionBegin { .. }
                | NativeIterationAction::CollectionElement { .. }
                | NativeIterationAction::CollectionPush { .. }
                | NativeIterationAction::CollectionFinish { .. }
                | NativeIterationAction::TextJoinFinish { .. }
                | NativeIterationAction::HistogramQueryStep { .. }
                | NativeIterationAction::HistogramFinish { .. }
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
        if self
            .prefix
            .as_ref()
            .is_some_and(|prefix| prefix.paused.is_some())
        {
            return Err(invalid("helper after native prefix checkpoint"));
        }
        if operation == super::program_abi::CLOSED_TAIL_END {
            let site = usize::try_from(site).map_err(|_| invalid("closed Tail site overflow"))?;
            return self.end_closed_tail(site, args);
        }
        if self.closed_tail.is_some() {
            return Err(invalid(
                "ordinary helper while closed Tail fuel is outstanding",
            ));
        }
        if matches!(operation, super::program_abi::PARALLEL_RUN
            | super::program_abi::PARALLEL_TAKE | super::program_abi::PARALLEL_YIELD
            | super::program_abi::PARALLEL_ENTER) {
            return self.parallel_helper(usize::try_from(site)
                .map_err(|_| invalid("parallel helper site overflow"))?, operation, args);
        }
        if matches!(
            operation,
            super::program_abi::PREFIX_PAUSE
                | super::program_abi::PREFIX_BORROWED_RESUME
                | super::program_abi::PREFIX_TAIL_RESUME
                | super::program_abi::PREFIX_SCANNED
        ) {
            return self.prefix_helper(site, operation, args);
        }
        // Native path interiors have no callback. A selected map operation
        // admits its own observable boundary before reading any operands or
        // mutating storage. Canonical/refused execution keeps ordinary visits.
        if (operation == super::program_abi::MAP_OWNER_CALL
                && self.retained_native_call(site as usize))
            || (operation == super::program_abi::MAP_OWNER_SPAN
                && self.retained_native_span(site as usize))
        {
            self.visit(site)?;
        }
        if self.control.previous.map(u64::from) != Some(site) {
            return Err(invalid("helper outside exact admitted visit"));
        }
        let site = site as usize;
        #[cfg(all(test, feature = "canonical-native-program"))]
        collection::before_span_helper(self, site)?;
        let descriptor = self.control.descriptor;
        if operation == super::program_abi::CLOSED_TAIL_BEGIN {
            return self.begin_closed_tail(site, args);
        }
        if matches!(operation, super::program_abi::MAP_OWNER_BEGIN
            | super::program_abi::MAP_OWNER_CALL | super::program_abi::MAP_OWNER_END
            | super::program_abi::MAP_OWNER_SPAN) {
            return self.retained_map_helper(site, operation, args);
        }
        if self.retained_native_action(site) || self.retained_native_call(site) || self.retained_native_exit(site)
            || self.retained_native_span(site) {
            return Err(invalid("ordinary helper at retained native map site"));
        }
        if operation == super::program_abi::BORROWED_STRINGS_PREPARE {
            return self.prepare_borrowed_strings(site, args);
        }
        if operation == super::program_abi::BORROWED_STRING_FAILURE {
            return self.borrowed_string_failure(site, args);
        }
        if self.borrowed_action(site).is_some() {
            return Err(invalid("ordinary helper at native borrowed String site"));
        }
        if descriptor.scalar_aggregates.action(site).is_some() {
            return Err(invalid("helper at native-only aggregate site"));
        }
        if operation == super::program_abi::WINDOW_PREPARE {
            self.prepare_window_region(site, args)
        } else if operation == super::program_abi::WINDOW_FINISH {
            self.finish_window_region(site, args)
        } else if operation == super::program_abi::STRING_MAP_PREPARE {
            self.prepare_string_map_region(site, args)
        } else if operation == super::program_abi::STRING_MAP_FINISH {
            self.finish_string_map_region(site, args)
        } else if self.window_region.is_some() {
            Err(invalid("helper while compiled String loans are active"))
        } else if operation == 0xffff_ffff_ffff_fff0 {
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
            if *observation == Observation::CanonicalValue {
                let NativeIterationAction::Structural { output, .. } =
                    descriptor.plan.blocks()[site].action()
                else {
                    unreachable!()
                };
                let prior = if self
                    .arena
                    .current_retained_bytes(output.get(), &mut self.control.work)?
                    .is_some()
                {
                    Some(
                        self.arena
                            .copy_current(output.get(), &mut self.control.work)?,
                    )
                } else {
                    None
                };
                let result = mech_engine::native_artifact::native_project_structural_value(
                    &descriptor.plan,
                    site,
                    &descriptor.schemas,
                    &root,
                    prior.as_ref(),
                    self.limits,
                    &self.library,
                )
                .map_err(|error| NativeProgramExecutionError::Library {
                    origin: descriptor.plan.blocks()[site].origin(),
                    error,
                })?;
                self.arena
                    .put(output.get(), &result, &mut self.control.work)?;
                self.consumed = true;
                return descriptor.plan.values()[output.get() as usize]
                    .scalar_kind()
                    .map(|kind| bits(&result, kind))
                    .transpose();
            }
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
            if let Some(result) = self.line_matrix_call(site, words)? {
                self.consumed = true;
                return Ok(result);
            }
            if self.captured_line_element_call(site, words)? {
                self.consumed = true;
                return Ok(None);
            }
            if self.selected_join_call(site, words)? {
                self.consumed = true;
                return Ok(None);
            }
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
            let outcome = self.call_caches.invoke(
                site,
                call,
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
                && matches!(
                    descriptor.plan.iterations()[*iteration as usize],
                    mech_engine::native_artifact::NativeControlLoop::Tail(_)
                )
            {
                self.retire_tail_locals(*iteration, RetirementKind::TailLatch)?;
            }
            self.consumed = true;
            Ok(None)
        } else if operation == COLLECTION {
            if self.consumed || args[2] != 0 {
                return Err(invalid("duplicate collection helper"));
            }
            let mut state = self
                .collections
                .take()
                .ok_or_else(|| invalid("collection helper without state"))?;
            let result = state.helper(self, site, [args[0], args[1]]);
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
        if self
            .prefix
            .as_ref()
            .is_some_and(|prefix| prefix.paused.is_some())
        {
            return Err(invalid("visit after native prefix checkpoint"));
        }
        if self.closed_tail.is_some() {
            return Err(invalid(
                "ordinary visit while closed Tail fuel is outstanding",
            ));
        }
        if self.window_region.is_some() {
            return Err(invalid("compiled String loans escaped their region"));
        }
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
        let previous = if let Some((from, to)) = self.map_owners.closed_exit {
            if self.control.previous != Some(from) || site != u64::from(to) {
                return Err(invalid("visit outside closed map exit frontier"));
            }
            Some(from)
        } else if let Some(previous) = self.control.previous
            && let Some(group) = self.control.descriptor.map_admission.group(previous as usize)
            && self.map_owners.active(group.owner)
        {
            if previous != group.start() { return Err(invalid("native path lost its entry")); }
            Some(group.predecessor(u32::try_from(site)
                .map_err(|_| invalid("map path frontier overflow"))?)
                .ok_or_else(|| invalid("visit outside prepaid native path frontier"))?)
        } else { self.control.previous };
        // A native aggregate latch has no managed move callback. Retire the
        // OTHER managed loop locals after that generated copy, before the next
        // visit, exactly as the managed latch used to do. Aggregate words own
        // no heap payload and are absent from the retirement inventory.
        if let Some(previous) = previous
            && (self
                .control
                .descriptor
                .scalar_aggregates
                .action(previous as usize)
                .is_some() || self.retained_native_action(previous as usize))
            && let NativeIterationAction::Latch { iteration } =
                self.control.descriptor.plan.blocks()[previous as usize].action()
            && matches!(
                self.control.descriptor.plan.iterations()[*iteration as usize],
                mech_engine::native_artifact::NativeControlLoop::Tail(_)
            )
        {
            self.retire_tail_locals(*iteration, RetirementKind::TailLatch)?;
        }
        let index =
            usize::try_from(site).map_err(|_| invalid("program visit site outside descriptor"))?;
        if let Some(worker) = &self.worker
            && !self.control.descriptor.parallel.regions.get(worker.region)
                .is_some_and(|region| region.contains(index as u32)) {
            return Err(invalid("worker visited outside its outlined source body"));
        }
        let block = self
            .control
            .descriptor
            .plan
            .blocks()
            .get(index)
            .ok_or_else(|| invalid("program visit site outside descriptor"))?;
        self.validate_selected_join_boundary(index)?;
        if let Some(state) = &self.collections {
            state.validate_span_boundary(block.action())?;
        }
        let group = self.control.descriptor.map_admission.group(index)
            .filter(|group| self.map_owners.active(group.owner));
        if group.is_some_and(|group| u64::from(group.start()) != site) {
            return Err(invalid("ordinary visit inside admitted scalar group"));
        }
        if previous.is_none() && self.worker.is_some() {
            let worker = self.worker.as_ref().unwrap();
            let region = self.control.descriptor.parallel.regions.get(worker.region)
                .ok_or_else(|| invalid("worker entry proof missing"))?;
            if site != u64::from(region.begin) || group.is_some() {
                return Err(invalid("worker entry outside constructor proof"));
            }
            self.control.visit_outlined_entry(site, region.begin)?;
        } else {
            self.control.visit_from(site, group.map(|group| group.work), previous)?;
        }
        self.map_owners.closed_exit = None;
        let descriptor = self.control.descriptor;
        if let Some(mech_engine::native_artifact::NativeBorrowedStringAction::Equal { .. }) =
            self.borrowed_action(index)
        {
            self.library.charge(32, 32).map_err(|error| {
                NativeProgramExecutionError::Collection {
                    origin: block.origin(),
                    error,
                }
            })?;
        }
        if let super::iteration_abi::Progress::Exit(iteration) = descriptor.visits[index].progress
            && self.borrowed_strings.as_ref().is_some_and(|state| {
                descriptor.borrowed_strings.regions[state.region]
                    .proof
                    .iteration()
                    == iteration
            })
        {
            self.borrowed_strings = None;
        }
        if let super::iteration_abi::Progress::Exit(iteration) =
            descriptor.visits[site as usize].progress
            && matches!(
                descriptor.plan.iterations()[iteration as usize],
                mech_engine::native_artifact::NativeControlLoop::Tail(_)
            )
        {
            // Keep only the final return until the following generated copy.
            self.retire_tail_locals(iteration, RetirementKind::TailExit)?;
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
        if self.control.completed && self.has_retained_maps() {
            return Err(invalid("completion with an active retained map owner"));
        }
        self.consumed = false;
        // Keep the actual admitted anchor. The next callback must belong to
        // this prepaid path's exact frontier; interior failures retain their
        // original source sites. No synthetic per-instruction visits occur.
        Ok(())
    }

    fn failure_site(&self, raw: u64) -> Result<u32, NativeProgramExecutionError> {
        let previous = self.control.previous.ok_or_else(|| invalid("program failure before visit"))?;
        let site = u32::try_from(raw).map_err(|_| invalid("program failure origin mismatch"))?;
        if site == previous { return Ok(site); }
        let layout = &self.control.descriptor.map_admission;
        if let Some(group) = layout.group(previous as usize)
            && self.map_owners.active(group.owner) && group.start() == previous
            && group.sites.contains(&site) {
            return Ok(site);
        }
        Err(invalid("program failure origin mismatch"))
    }

    fn retire_tail_locals(
        &mut self,
        iteration: u32,
        kind: RetirementKind,
    ) -> Result<(), NativeProgramExecutionError> {
        self.retire_map_keys(iteration)?;
        let slots = self
            .control
            .descriptor
            .retirements
            .slots(self.retirements, iteration, kind)
            .map_err(invalid)?;
        let scan = amount(slots.len(), 16, 64)?;
        self.control
            .work
            .charge(NativeExecutionPhase::Setup, scan)?;
        for &slot in slots {
            self.arena
                .retire_current(slot as u32, &mut self.control.work)?;
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
    let outcome = context.visit(site);
    match outcome {
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
    let expected = if turn.text_capture().is_some() {
        0
    } else {
        descriptor.plan.observations().len()
    };
    if turn.observations().len() != expected {
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
mod histogram_query_observation_tests {
    //! Header observations around the SAME actual emitted entry and real helper.
    use super::*;
    use collection::QueryDemand;
    use mech_core::{
        ManagedMemoryReservation, MemoryBudgetDimension as Dimension, MemoryRuntimeError,
        ResidentKernelError,
    };
    use mech_engine::native_artifact::{
        NativeCollectionResult, NativeHistogramQueryStepKind as Step,
    };
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    const SOURCE: &str = "+> map\nlookup(items<[string]>, query<string>) => (out<u64>) :=\n  words := [part | part <- items]\n  counts := map/histogram(words)\n  keys<{string}> := counts_keys\n  count := (query ∈ keys) ? | true => counts{query} | false => 0u64.\n  out := count.\nlookup(source<[string]>, query<string>)\n";
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Target {
        collection: u32,
        step: Option<u32>,
        accepted: u64,
        scalar_bytes: u64,
    }
    impl Target {
        fn of(d: &QueryDemand) -> Self {
            Self {
                collection: d.collection,
                step: d.step,
                accepted: d.accepted,
                scalar_bytes: d.scalar_bytes,
            }
        }
    }
    #[derive(Clone, Copy, Debug)]
    struct ScalarBoundary {
        target: Target,
        envelope: u64,
        available: u64,
        previous_output_bytes: Option<u64>,
        scalar_owners: (u64, u64),
        memory_before: u64,
        memory_pressured: u64,
        work_before: u64,
        pressure_failed: bool,
    }
    #[derive(Clone, Copy, Debug)]
    struct Row {
        before: QueryDemand,
        after: Option<QueryDemand>,
        status: u64,
        memory_before: u64,
        memory_after: u64,
        work_before: u64,
        work_after: u64,
        live_after: u64,
        scalar_owners_after: (u64, u64),
        scalar: Option<ScalarBoundary>,
    }
    thread_local! {
        static ROWS: std::cell::RefCell<Vec<Row>> = const { std::cell::RefCell::new(Vec::new()) };
        static PRESSURE: std::cell::Cell<Option<(Target, bool)>> = const { std::cell::Cell::new(None) };
        static SCALAR: std::cell::Cell<Option<ScalarBoundary>> = const { std::cell::Cell::new(None) };
    }
    /// This safe test-only header hook runs immediately before the unchanged
    /// Context::scalar reservation E; selected-stage lookup allocates nothing.
    /// Ballast is an ordinary reservation on the SAME immutable account; no
    /// quota, production price, body, generation or Value owner is changed.
    /// It returns the guard so pressure encloses only that real constructor.
    /// There are deliberately no assertions that could unwind through C.
    pub(super) fn before_scalar(
        state: &collection::State,
        ctx: &Context<'_, '_>,
        id: u32,
        ordinal: u32,
        previous_output_bytes: Option<u64>,
    ) -> Option<ManagedMemoryReservation> {
        let mut demand = state.test_query_state(id)?;
        demand.step = Some(ordinal);
        let target = Target::of(&demand);
        let memory = ctx.library.memory_budget();
        let memory_before = memory.used_bytes();
        let envelope = ctx.control.descriptor.scalar_bytes;
        let mut ballast = None;
        let mut pressure_failed = false;
        if let Some((requested, short)) = PRESSURE.with(std::cell::Cell::get)
            && requested == target
        {
            let available = envelope.checked_sub(u64::from(short));
            match available.and_then(|available| {
                memory
                    .limit_bytes()
                    .checked_sub(memory_before)?
                    .checked_sub(available)
            }) {
                Some(amount) => match memory.reserve_capacity(amount) {
                    Ok(reservation) => ballast = Some(reservation),
                    Err(_) => pressure_failed = true,
                },
                None => pressure_failed = true,
            }
        }
        let memory_pressured = memory.used_bytes();
        SCALAR.with(|slot| {
            slot.set(Some(ScalarBoundary {
                target,
                envelope,
                available: memory.limit_bytes().saturating_sub(memory_pressured),
                previous_output_bytes,
                scalar_owners: state.test_query_scalar_owners(),
                memory_before,
                memory_pressured,
                work_before: ctx.control.work.used(),
                pressure_failed,
            }))
        });
        ballast
    }
    // No assertion may unwind through C. The observer neither changes the policy
    // nor chooses any replacement work; pressure lasts around only the target.
    unsafe extern "C" fn observe(
        context: *mut c_void,
        site: u64,
        operation: u64,
        first: u64,
        second: u64,
        third: u64,
        result: *mut u64,
    ) -> u64 {
        SCALAR.with(|slot| slot.set(None));
        let before = (|| {
            // SAFETY: execute retains this initialized Context for the actual JIT.
            let ctx = unsafe { &*context.cast::<Context<'_, '_>>() };
            let demand = ctx
                .collections
                .as_ref()?
                .test_query_demand(ctx, usize::try_from(site).ok()?)?;
            Some((demand, ctx.library.memory_budget(), ctx.control.work.used()))
        })();
        let memory_before = before
            .as_ref()
            .map_or(0, |(_, memory, _)| memory.used_bytes());
        // SAFETY: every argument is unchanged from the actual generated call.
        let status =
            unsafe { super::helper(context, site, operation, first, second, third, result) };
        if let Some((demand, memory, work_before)) = before {
            // SAFETY: the real helper's exclusive borrow has ended.
            let ctx = unsafe { &*context.cast::<Context<'_, '_>>() };
            let after = ctx
                .collections
                .as_ref()
                .and_then(|state| state.test_query_state(demand.collection));
            let live_after = ctx
                .collections
                .as_ref()
                .map_or(0, |state| state.test_live_bytes());
            let scalar_owners_after = ctx
                .collections
                .as_ref()
                .map_or((0, 0), |state| state.test_query_scalar_owners());
            ROWS.with(|rows| {
                rows.borrow_mut().push(Row {
                    before: demand,
                    after,
                    status,
                    memory_before,
                    memory_after: memory.used_bytes(),
                    work_before,
                    work_after: ctx.control.work.used(),
                    live_after,
                    scalar_owners_after,
                    scalar: SCALAR.with(std::cell::Cell::take),
                })
            });
        }
        status
    }
    fn observed(
        jit: &super::super::JitProgramArtifact,
        args: &[Value],
        outputs: &mut [Option<Value>],
        limits: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
        pressure: Option<(Target, bool)>,
    ) -> (
        Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
        Vec<Row>,
    ) {
        ROWS.with(|rows| rows.borrow_mut().clear());
        PRESSURE.with(|slot| slot.set(pressure));
        let entry = jit.test_checked_entry();
        let outcome = execute(
            jit.descriptor(),
            |raw| {
                // SAFETY: same actual entry, frame and every argument; restore the hook
                // before the production immutable-frame checks observe the return.
                let original = unsafe { (*raw).helper };
                unsafe { (*raw).helper = observe };
                let status = unsafe { entry(raw) };
                unsafe { (*raw).helper = original };
                status
            },
            args,
            outputs,
            limits,
            memory,
        );
        PRESSURE.with(|slot| slot.set(None));
        let rows = ROWS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
        (outcome, rows)
    }
    fn artifacts() -> [mech_engine::ProgramArtifact; 2] {
        artifacts_source(SOURCE)
    }
    fn artifacts_source(source: &str) -> [mech_engine::ProgramArtifact; 2] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x5155455259), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let original = mech_engine::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        [original, decoded]
    }
    fn caps() -> NativeProgramExecutionLimits {
        let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        // Explicit finite test policy, not a production default/cap change.
        library.max_compute_work = Some(134_217_728);
        library.max_comparison_work = Some(1_000_000);
        library.max_temporary_bytes = Some(128 * 1024 * 1024);
        NativeProgramExecutionLimits {
            execution: NativeExecutionLimits {
                max_work: 1_000_000_000,
                ..NativeExecutionLimits::default()
            },
            library,
        }
    }
    fn args(
        artifact: &mech_engine::ProgramArtifact,
        memory: &ManagedMemoryBudget,
        parts: &[String],
        query: &str,
    ) -> Vec<Value> {
        artifact
            .inputs()
            .iter()
            .map(|input| {
                let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                let (data, extents) = match name.as_str() {
                    "source" => (
                        ValueDataDraft::Matrix(
                            parts.iter().cloned().map(ValueDataDraft::String).collect(),
                        ),
                        Some([1, parts.len() as u64]),
                    ),
                    "query" => (ValueDataDraft::String(query.into()), None),
                    "word" => (ValueDataDraft::String(query.into()), None),
                    "indices" => (
                        ValueDataDraft::Matrix([0, 1, 2, 3].map(ValueDataDraft::U64).into()),
                        Some([1, 4]),
                    ),
                    "queries" => (
                        ValueDataDraft::Matrix(
                            vec![
                                ValueDataDraft::String(query.into()),
                                ValueDataDraft::String("later".into()),
                            ]
                            .into_boxed_slice(),
                        ),
                        Some([1, 2]),
                    ),
                    _ => panic!("unexpected {name}"),
                };
                let shape_values = extents
                    .map(|extents| {
                        mech_core::shape_for_resolved_extents(
                            artifact.schemas().get(input.schema).unwrap(),
                            &extents,
                        )
                        .unwrap()
                        .parameter_values()
                        .to_vec()
                        .into_boxed_slice()
                    })
                    .unwrap_or_default();
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values,
                    data,
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                    .unwrap();
                value.into_memory_budget(&mut reservation).unwrap()
            })
            .collect()
    }
    fn snapshots(
        artifact: &mech_engine::ProgramArtifact,
        outputs: &[Option<Value>],
    ) -> Vec<Box<[u8]>> {
        outputs
            .iter()
            .map(|value| {
                value
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap()
            })
            .collect()
    }
    fn row(rows: &[Row], target: Target) -> &Row {
        let rows = rows
            .iter()
            .filter(|row| Target::of(&row.before) == target)
            .collect::<Vec<_>>();
        assert_eq!(
            rows.len(),
            1,
            "actual target must be visited once: {target:?}"
        );
        rows[0]
    }
    fn unchanged(row: &Row) {
        let after = row
            .after
            .expect("denied helper keeps private state until unwind");
        assert_eq!(
            (after.accepted, after.matched, after.phase),
            (row.before.accepted, row.before.matched, row.before.phase)
        );
        assert_eq!(after.scalar_bytes, row.before.scalar_bytes);
        assert_eq!(after.live_bytes, row.before.live_bytes);
        assert_eq!(row.memory_after, row.memory_before);
        assert!(row.work_after >= row.work_before);
    }

    #[test]
    fn actual_query_last_comparison_and_scalar_local_caps_precede_mutation_and_recover() {
        for artifact in artifacts() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let sink = jit
                .descriptor()
                .plan()
                .collections()
                .iter()
                .find_map(|c| {
                    if let NativeCollectionResult::GuardedHistogramQuery(s) = c.result() {
                        Some(s)
                    } else {
                        None
                    }
                })
                .expect("actual demand selected");
            let lookup = sink
                .steps()
                .iter()
                .position(|step| step.kind() == Step::Lookup)
                .unwrap() as u32;
            let args = args(
                &artifact,
                &memory,
                &["é\0x".into(), "é\0x".into(), "later".into(), "é\0x".into()],
                "é\0x",
            );
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            let prior = snapshots(&artifact, &outputs);
            let stable = memory.used_bytes();
            let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            outcome.unwrap();
            let last = baseline
                .iter()
                .filter(|row| row.before.step.is_none())
                .max_by_key(|row| row.before.accepted)
                .unwrap();
            assert_eq!(last.before.accepted, 3, "no first-hit early exit");
            let scalar = baseline
                .iter()
                .find(|row| row.before.step == Some(lookup))
                .unwrap();
            let scalar_after = scalar.after.unwrap();
            for (target, dimension, required) in [
                (
                    Target::of(&last.before),
                    Dimension::ComparisonWork,
                    last.before.comparison_required,
                ),
                (
                    Target::of(&scalar.before),
                    Dimension::ComputeWork,
                    scalar_after.compute,
                ),
                (
                    Target::of(&scalar.before),
                    Dimension::ClonedBytes,
                    scalar_after.cloned,
                ),
                // Old canonical Bool + captured String + new canonical U64.
                (Target::of(&scalar.before), Dimension::RetainedNodes, 6),
            ] {
                for short in [false, true] {
                    let mut limits = caps();
                    let bound = required - u64::from(short);
                    match dimension {
                        Dimension::ComparisonWork => {
                            limits.library.max_comparison_work = Some(bound)
                        }
                        Dimension::ComputeWork => limits.library.max_compute_work = Some(bound),
                        Dimension::ClonedBytes => limits.library.max_cloned_bytes = Some(bound),
                        Dimension::TemporaryBytes => {
                            limits.library.max_temporary_bytes = Some(bound)
                        }
                        Dimension::RetainedNodes => limits.library.max_retained_nodes = Some(bound),
                        _ => unreachable!(),
                    }
                    let (outcome, rows) =
                        observed(&jit, &args, &mut outputs, limits, &memory, None);
                    let actual = row(&rows, target);
                    if short {
                        assert_ne!(actual.status, STATUS_OK);
                        unchanged(actual);
                        assert!(
                            matches!(outcome.unwrap_err(), NativeProgramExecutionError::Collection {
                            error: ResidentKernelError::ResourceLimit { dimension: actual, required: amount, limit, .. }, ..
                        } if actual == dimension && amount == required && limit == required - 1)
                        );
                    } else {
                        assert_eq!(
                            actual.status, STATUS_OK,
                            "later phases may still need larger caps: {outcome:?}"
                        );
                    }
                    assert_eq!(snapshots(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                    jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                        .unwrap();
                }
            }
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }

    #[test]
    fn actual_query_fixed_selected_stage_exact_one_short_keeps_spent_prefix_and_recovers() {
        for artifact in artifacts() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let (collection, sink) = jit
                .descriptor()
                .plan()
                .collections()
                .iter()
                .find_map(|c| match c.result() {
                    NativeCollectionResult::GuardedHistogramQuery(s) => Some((c, s)),
                    _ => None,
                })
                .unwrap();
            let ordinal = sink
                .steps()
                .iter()
                .position(|s| s.kind() == Step::Keys)
                .unwrap() as u32;
            let selected = jit.descriptor().calls[sink.steps()[ordinal as usize].site() as usize]
                .as_ref()
                .unwrap();
            assert_eq!(
                selected.template(),
                sink.steps()[ordinal as usize].template().unwrap()
            );
            assert!(
                selected.binding_work() > 128,
                "the real selected binding was not replaced"
            );
            let args = args(
                &artifact,
                &memory,
                &["é\0x".into(), "later".into(), "é\0x".into()],
                "é\0x",
            );
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            let prior = snapshots(&artifact, &outputs);
            let stable = memory.used_bytes();
            // A SECOND funded invocation includes exactly the same old-publication
            // retirement prepayment as all exact/short invocations below.
            let (outcome, rows) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            outcome.unwrap();
            let funded = rows
                .iter()
                .find(|r| r.before.step == Some(ordinal))
                .unwrap();
            let target = Target::of(&funded.before);
            let after = funded.after.unwrap();
            assert_eq!(
                after.compute,
                funded.before.compute.checked_add(128).unwrap()
            );
            assert_eq!(
                funded.before.phase,
                Some(mech_engine::native_artifact::NativeHistogramQueryPhase::HistogramReady)
            );
            assert_eq!(
                after.phase,
                Some(mech_engine::native_artifact::NativeHistogramQueryPhase::KeysReady)
            );
            assert_eq!(
                (after.scalar_bytes, after.live_bytes),
                (funded.before.scalar_bytes, funded.before.live_bytes)
            );
            assert_eq!(funded.before.reservation_bytes, 0);
            assert_eq!(funded.memory_after, funded.memory_before);
            // The saved-generation arena read is honestly spent BEFORE the
            // atomic fixed-stage debit. It is not a free helper or a refund.
            let prefix = funded.work_after.checked_sub(128).unwrap();
            assert!(prefix > funded.work_before && funded.work_before > 0);
            for global in [false, true] {
                let exact = if global {
                    funded.work_after
                } else {
                    after.compute
                };
                for short in [false, true] {
                    let mut limits = caps();
                    if global {
                        limits.execution.max_work = exact - u64::from(short);
                    } else {
                        limits.library.max_compute_work = Some(exact - u64::from(short));
                    }
                    let (outcome, rows) =
                        observed(&jit, &args, &mut outputs, limits, &memory, None);
                    let actual = row(&rows, target); // Earlier denial is never accepted as this boundary.
                    assert_eq!(actual.work_before, funded.work_before);
                    if short {
                        assert_ne!(actual.status, STATUS_OK);
                        unchanged(actual);
                        assert_eq!(
                            actual.work_after, prefix,
                            "denial keeps all prefix work without debit/refund"
                        );
                        let error = outcome.unwrap_err();
                        if global {
                            assert!(
                                matches!(error, NativeProgramExecutionError::Base(
                                NativeExecutionError::WorkLimit { phase: NativeExecutionPhase::Setup, required, limit }
                            ) if required == exact && limit == exact - 1),
                                "{error:?}"
                            );
                        } else {
                            assert!(
                                matches!(error, NativeProgramExecutionError::Collection {
                                origin, error: ResidentKernelError::ResourceLimit {
                                    phase: mech_core::ResidentResourcePhase::Admission,
                                    dimension: Dimension::ComputeWork, required, limit,
                                }
                            } if origin == collection.origin() && required == exact && limit == exact - 1),
                                "{error:?}"
                            );
                        }
                    } else {
                        assert_eq!(actual.status, STATUS_OK, "{outcome:?}");
                        assert_eq!(actual.work_after, funded.work_after);
                        let admitted = actual.after.unwrap();
                        assert_eq!(
                            (admitted.compute, admitted.phase),
                            (after.compute, after.phase)
                        );
                        if let Err(error) = outcome {
                            assert!(
                                match &error {
                                    NativeProgramExecutionError::Control(
                                        NativeIterationExecutionError::Work {
                                            required, limit, ..
                                        },
                                    ) if global => *required > *limit && *limit == exact,
                                    NativeProgramExecutionError::Collection {
                                        origin,
                                        error:
                                            ResidentKernelError::ResourceLimit {
                                                phase: mech_core::ResidentResourcePhase::Admission,
                                                dimension: Dimension::ComputeWork,
                                                required,
                                                limit,
                                            },
                                    } if !global =>
                                        *origin == collection.origin()
                                            && *required > *limit
                                            && *limit == exact,
                                    _ => false,
                                },
                                "fixed stage succeeded; only a truthful later work denial is permitted: {error:?}"
                            );
                        }
                    }
                    assert_eq!(snapshots(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                    jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                        .unwrap();
                    assert_eq!(snapshots(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                }
            }
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }

    #[test]
    fn actual_query_lookup_local_temporary_exact_one_short_reaches_selected_stage_and_recovers() {
        // An admitted numeric Matrix yields real scalar words without an
        // earlier Range binder or managed String element-copy reservation.
        // Every visit still yields the genuine captured String. With immutable
        // stage association proved once, Lookup's actual peak is live + E for
        // the unchanged canonical scalar constructor, including the old Bool.
        let source = "+> map\nlookup(word<string>, query<string>, indices<[u64]>) => (out<u64>) :=\n  words := [word | index <- indices]\n  counts := map/histogram(words)\n  keys<{string}> := counts_keys\n  count := (query ∈ keys) ? | true => counts{query} | false => 0u64.\n  out := count.\nlookup(word<string>, query<string>, indices<[u64]>)\n";
        for artifact in artifacts_source(source) {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let (collection, sink) = jit
                .descriptor()
                .plan()
                .collections()
                .iter()
                .find_map(|collection| match collection.result() {
                    NativeCollectionResult::GuardedHistogramQuery(sink) => Some((collection, sink)),
                    _ => None,
                })
                .expect("actual selected query");
            let lookup = sink
                .steps()
                .iter()
                .position(|step| step.kind() == Step::Lookup)
                .unwrap() as u32;
            for &generator in collection.generators() {
                let mech_engine::native_artifact::NativeControlLoop::Collection(generator) =
                    &jit.descriptor().plan().iterations()[generator as usize]
                else {
                    panic!("actual collection source");
                };
                let source = generator
                    .source()
                    .materialized_value()
                    .expect("fixture must actually use its admitted numeric Matrix");
                assert!(matches!(
                    jit.descriptor().plan().values()[source.get() as usize].definition().body(),
                    mech_core::SchemaBody::Matrix { element, .. }
                        if element.as_ref() == &mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
                ));
            }
            assert!(
                !jit.descriptor()
                    .plan()
                    .blocks()
                    .iter()
                    .any(|block| matches!(
                        block.action(),
                        NativeIterationAction::PrepareU64Range { .. }
                            | NativeIterationAction::LibraryCall { .. }
                    )),
                "no unrelated earlier selected binder may mask the actual scalar boundary"
            );
            let envelope = jit.descriptor().scalar_bytes;
            let args = args(&artifact, &memory, &[], "é\0x");
            let word = args[artifact
                .inputs()
                .iter()
                .position(|input| {
                    mech_engine::decode_source_input_name(&input.name).unwrap() == "word"
                })
                .unwrap()]
            .memory_budget_retained_bytes(&memory)
            .unwrap();
            assert!(
                jit.descriptor().scalar_bytes > word,
                "earlier captured-yield peak must fit"
            );
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            assert_eq!(
                outputs
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::U64(4)
            );
            let prior = snapshots(&artifact, &outputs);
            let stable = memory.used_bytes();
            let (outcome, rows) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            outcome.unwrap();
            assert_eq!(
                rows.iter().filter(|row| row.before.step.is_none()).count(),
                4,
                "every numeric candidate still yields and compares"
            );
            let scalar = rows
                .iter()
                .find(|row| row.before.step == Some(lookup))
                .expect("actual Lookup scalar stage");
            assert_eq!(scalar.before.accepted, 4);
            assert_eq!(scalar.before.matched, 4);
            assert!(
                scalar.before.scalar_bytes > 0,
                "old Bool owner remains live"
            );
            assert_eq!(scalar.before.reservation_bytes, envelope);
            let target = Target::of(&scalar.before);
            let required = scalar.before.temporary_required;
            assert_eq!(required, scalar.before.live_bytes + envelope);
            let earlier_peak = rows
                .iter()
                .take_while(|row| Target::of(&row.before) != target)
                .map(|row| row.before.temporary_required)
                .max()
                .unwrap();
            assert!(
                earlier_peak < required,
                "Lookup must be a genuine later peak, not an unreachable one-short cap"
            );
            for short in [false, true] {
                let mut limits = caps();
                limits.library.max_temporary_bytes = Some(required - u64::from(short));
                let (outcome, rows) = observed(&jit, &args, &mut outputs, limits, &memory, None);
                let actual = row(&rows, target); // Fails on any earlier setup/body/element denial.
                if short {
                    assert_ne!(actual.status, STATUS_OK);
                    unchanged(actual);
                    assert!(
                        matches!(outcome.unwrap_err(),NativeProgramExecutionError::Collection{error:ResidentKernelError::ResourceLimit{dimension:Dimension::TemporaryBytes,required:amount,limit,..},..}if amount==required&&limit==required-1)
                    );
                } else {
                    assert_eq!(actual.status, STATUS_OK, "{outcome:?}");
                }
                assert_eq!(snapshots(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                    .unwrap();
                assert_eq!(snapshots(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
            }
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }

    #[test]
    fn actual_query_scalar_global_reservation_exact_and_one_short_has_no_future_credit() {
        let source = SOURCE.replace(
            "lookup(source<[string]>, query<string>)",
            "[lookup(source<[string]>, query) | query <- queries<[string]>]",
        );
        for artifact in artifacts_source(&source) {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let args = args(
                &artifact,
                &memory,
                &["hit".into(), "hit".into(), "later".into()],
                "hit",
            );
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            let prior = snapshots(&artifact, &outputs);
            let stable = memory.used_bytes();
            let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            outcome.unwrap();
            let sink = jit
                .descriptor()
                .plan()
                .collections()
                .iter()
                .find_map(|collection| {
                    if let NativeCollectionResult::GuardedHistogramQuery(sink) = collection.result()
                    {
                        Some(sink)
                    } else {
                        None
                    }
                })
                .unwrap();
            let cleanup = sink
                .steps()
                .iter()
                .position(|step| step.kind() == Step::Cleanup)
                .unwrap() as u32;
            let cleanups = baseline
                .iter()
                .filter(|row| row.before.step == Some(cleanup))
                .collect::<Vec<_>>();
            assert_eq!(cleanups.len(), 2);
            for row in cleanups {
                assert!(row.after.is_none());
                assert_eq!(
                    row.memory_after, row.memory_before,
                    "cleanup must not return physical scalar/source owner capacity"
                );
                assert!(row.scalar_owners_after.0 > 0 && row.scalar_owners_after.1 > 0);
                assert_eq!(
                    row.live_after,
                    row.before.live_bytes - row.before.query_bytes,
                    "cleanup ends only the capture pin; scalar roots stay in all following peaks"
                );
                assert!(row.live_after >= row.scalar_owners_after.0);
            }
            for kind in [Step::Membership, Step::Lookup] {
                let ordinal = sink
                    .steps()
                    .iter()
                    .position(|step| step.kind() == kind)
                    .unwrap() as u32;
                let visits = baseline
                    .iter()
                    .filter(|row| row.before.step == Some(ordinal))
                    .collect::<Vec<_>>();
                assert_eq!(
                    visits.len(),
                    2,
                    "both fresh outer queries must run the same generic scalar site"
                );
                let previous = visits[0].before.scalar_bytes;
                let target = visits[1];
                assert!(
                    target.before.scalar_bytes > previous,
                    "second query must retain both old Bool/U64 roots before replacement"
                );
                assert!(target.before.reservation_bytes > 0 && target.before.query_bytes > 0);
                let scalar_bytes = jit.descriptor().scalar_bytes;
                let baseline_scalar = target.scalar.expect("actual scalar constructor boundary");
                assert_eq!(baseline_scalar.envelope, scalar_bytes);
                assert!(
                    baseline_scalar
                        .previous_output_bytes
                        .is_some_and(|bytes| bytes > 0)
                );
                assert!(baseline_scalar.scalar_owners.0 >= target.before.scalar_bytes);
                assert!(baseline_scalar.scalar_owners.1 >= 4);
                assert!(!baseline_scalar.pressure_failed);
                let target = Target::of(&target.before);
                assert_eq!(baseline_scalar.target, target);
                for short in [false, true] {
                    let (outcome, rows) = observed(
                        &jit,
                        &args,
                        &mut outputs,
                        caps(),
                        &memory,
                        Some((target, short)),
                    );
                    let actual = row(&rows, target);
                    let boundary = actual.scalar.expect("reached unchanged Context::scalar");
                    assert_eq!(boundary.target, target);
                    assert!(
                        !boundary.pressure_failed,
                        "ballast installation must succeed"
                    );
                    assert_eq!(boundary.envelope, scalar_bytes);
                    assert_eq!(boundary.available, scalar_bytes - u64::from(short));
                    assert!(
                        boundary
                            .previous_output_bytes
                            .is_some_and(|bytes| bytes > 0)
                    );
                    assert_eq!(boundary.scalar_owners, baseline_scalar.scalar_owners);
                    assert!(boundary.memory_pressured >= boundary.memory_before);
                    assert!(boundary.work_before >= actual.work_before);
                    if short {
                        assert_ne!(actual.status, STATUS_OK);
                        unchanged(actual);
                        assert!(matches!(
                            outcome.unwrap_err(),
                            NativeProgramExecutionError::Base(NativeExecutionError::Memory {
                                error: MemoryRuntimeError::BudgetExceeded {
                                    operation: None, requested, limit,
                                },
                            }) if limit == memory.limit_bytes() && requested == limit + 1
                        ));
                    } else {
                        outcome.unwrap();
                        assert_eq!(actual.status, STATUS_OK);
                    }
                    assert_eq!(snapshots(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                }
            }
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }

    #[test]
    fn actual_high_diversity_query_retains_no_cardinality_sized_keys_or_payloads() {
        for artifact in artifacts() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            assert!(
                jit.descriptor()
                    .plan()
                    .collections()
                    .iter()
                    .any(|c| matches!(
                        c.result(),
                        NativeCollectionResult::GuardedHistogramQuery(_)
                    ))
            );
            let parts = (0..256)
                .map(|index| format!("key-{index:014}"))
                .collect::<Vec<_>>();
            let args = args(&artifact, &memory, &parts, &parts[0]);
            let mut outputs = vec![None; artifact.outputs().len()];
            let (outcome, rows) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            outcome.unwrap();
            let pushes = rows
                .iter()
                .filter(|row| row.before.step.is_none())
                .collect::<Vec<_>>();
            assert_eq!(pushes.len(), parts.len());
            assert_eq!(
                pushes.last().unwrap().after.unwrap().accepted,
                parts.len() as u64
            );
            assert!(
                pushes
                    .iter()
                    .all(|row| row.before.live_bytes == pushes[0].before.live_bytes)
            );
            assert!(
                pushes
                    .iter()
                    .all(|row| row.before.query_bytes == pushes[0].before.query_bytes)
            );
            assert_eq!(
                outputs
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::U64(1)
            );
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod histogram_observation_tests {
    //! INLINE into program_runtime.rs's existing audited FFI boundary only.
    //! The checked generated entry and real helper remain the sole execution path.
    use super::*;
    use collection::{HistogramDemand as Demand, HistogramStage as Stage, HistogramVisit as Visit};
    use mech_core::{
        MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase,
    };
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    #[derive(Clone, Copy)]
    struct Pressure {
        demand: Demand,
        short: bool,
    }
    #[derive(Clone, Copy, Debug)]
    struct Row {
        visit: Visit,
        demand: Option<Demand>,
        after: Option<(u64, u64, u64, u64)>,
        memory_before: u64,
        memory_after: u64,
        work_before: u64,
        work_after: u64,
        global_remaining: Option<u64>,
        status: u64,
    }
    thread_local! {
        static ROWS:std::cell::RefCell<Vec<Row>>=const{std::cell::RefCell::new(Vec::new())};
        static PRESSURE:std::cell::Cell<Option<Pressure>>=const{std::cell::Cell::new(None)};
    }
    unsafe extern "C" fn observe(
        context: *mut c_void,
        site: u64,
        operation: u64,
        first: u64,
        second: u64,
        third: u64,
        result: *mut u64,
    ) -> u64 {
        collection::State::test_histogram_clear_observation();
        let before = (|| {
            // SAFETY: execute retains the synchronous Context; the shared borrow
            // ends before delegating unchanged arguments to the exclusive helper.
            let ctx = unsafe { &*context.cast::<Context<'_, '_>>() };
            let visit = ctx
                .collections
                .as_ref()?
                .test_histogram_visit(ctx, usize::try_from(site).ok()?)?;
            Some((visit, ctx.library.memory_budget(), ctx.control.work.used()))
        })();
        let (mut ballast, mut remaining, mut pressure_failed) = (None, None, false);
        if let Some((visit, memory, _)) = &before {
            if let Some(pressure) = PRESSURE.with(std::cell::Cell::get)
                && *visit == pressure.demand.visit
            {
                let free = pressure
                    .demand
                    .reservation_bytes
                    .saturating_sub(u64::from(pressure.short));
                match memory
                    .limit_bytes()
                    .checked_sub(memory.used_bytes())
                    .and_then(|available| available.checked_sub(free))
                {
                    Some(amount) => match memory.reserve_capacity(amount) {
                        Ok(token) => {
                            ballast = Some(token);
                            remaining = Some(free);
                        }
                        Err(_) => pressure_failed = true,
                    },
                    None => pressure_failed = true,
                }
            }
        }
        let memory_before = before.as_ref().map(|(_, memory, _)| memory.used_bytes());
        // SAFETY: delegate the identical generated call and result pointer. No
        // observer assertion can unwind through this C boundary.
        let status =
            unsafe { super::helper(context, site, operation, first, second, third, result) };
        if let Some((visit, memory, work_before)) = before {
            // SAFETY: the delegated exclusive helper borrow has ended.
            let ctx = unsafe { &*context.cast::<Context<'_, '_>>() };
            let demand = collection::State::test_histogram_take_observation();
            let after = ctx
                .collections
                .as_ref()
                .and_then(|state| state.test_histogram_storage(visit.collection));
            ROWS.with(|rows| {
                rows.borrow_mut().push(Row {
                    visit,
                    demand,
                    after,
                    memory_before: memory_before.unwrap_or(0),
                    memory_after: memory.used_bytes(),
                    work_before,
                    work_after: ctx.control.work.used(),
                    global_remaining: remaining,
                    status: if pressure_failed { u64::MAX } else { status },
                })
            });
        }
        drop(ballast); // Pressure ends before every later lifecycle/publication phase.
        status
    }
    fn observed(
        jit: &super::super::JitProgramArtifact,
        args: &[Value],
        outputs: &mut [Option<Value>],
        caps: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
        pressure: Option<Pressure>,
    ) -> (
        Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
        Vec<Row>,
    ) {
        ROWS.with(|rows| rows.borrow_mut().clear());
        PRESSURE.with(|slot| slot.set(pressure));
        let entry = jit.test_checked_entry();
        let outcome = execute(
            jit.descriptor(),
            |raw| {
                // SAFETY: the JIT owner is alive; restore immutable helper transport
                // before execute checks the frame. This does not replace computation.
                let previous = unsafe { (*raw).helper };
                unsafe { (*raw).helper = observe };
                let result = unsafe { entry(raw) };
                unsafe { (*raw).helper = previous };
                result
            },
            args,
            outputs,
            caps,
            memory,
        );
        PRESSURE.with(|slot| slot.set(None));
        let rows = ROWS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
        (outcome, rows)
    }
    fn artifacts() -> [mech_engine::ProgramArtifact; 2] {
        let source = "+> map\nmap/histogram([part | part <- source<[string]>])\n";
        artifacts_source(source)
    }
    fn artifacts_source(source: &str) -> [mech_engine::ProgramArtifact; 2] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x4849535444454d), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let original = mech_engine::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        [original, decoded]
    }
    fn caps() -> NativeProgramExecutionLimits {
        let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        library.max_compute_work = Some(134_217_728);
        library.max_comparison_work = Some(4_000_000);
        library.max_temporary_bytes = Some(128 * 1024 * 1024);
        NativeProgramExecutionLimits {
            execution: NativeExecutionLimits {
                max_work: 10_000_000_000,
                ..NativeExecutionLimits::default()
            },
            library,
        }
    }
    fn args(
        artifact: &mech_engine::ProgramArtifact,
        scalar_bytes: u64,
        memory: &ManagedMemoryBudget,
    ) -> Vec<Value> {
        // Choose actual vector geometry, not a pricing estimate. The last growth's
        // new backing strictly dominates the earlier source-element envelope, so
        // exact/one-short Temporary caps reach this real preflight, unlike the
        // earlier three-yield TextJoin fixture. Keys are small, sorted and distinct.
        let capacity = (scalar_bytes / collection::State::test_histogram_entry_bytes() + 128)
            .next_power_of_two()
            .max(1024);
        assert!(
            capacity <= 8192,
            "fixture must stay a small boundary test: {scalar_bytes}"
        );
        artifact
            .inputs()
            .iter()
            .map(|input| {
                let data = ValueDataDraft::Matrix(
                    (0..=capacity)
                        .map(|index| ValueDataDraft::String(format!("é\0{index:08x}")))
                        .collect(),
                );
                let shape_values = mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[1, capacity + 1],
                )
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice();
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values,
                    data,
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut token = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                    .unwrap();
                value.into_memory_budget(&mut token).unwrap()
            })
            .collect()
    }
    fn snapshot(
        artifact: &mech_engine::ProgramArtifact,
        outputs: &[Option<Value>],
    ) -> Vec<Box<[u8]>> {
        outputs
            .iter()
            .map(|value| {
                value
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap()
            })
            .collect()
    }
    fn captured_binding_peak(jit: &super::super::JitProgramArtifact) -> u64 {
        // Fixture geometry uses the actual maintained bindings, including the
        // range's three-port envelope. It does not change any runtime price.
        let plan = jit.descriptor().plan();
        assert_eq!(
            plan.collections()
                .iter()
                .filter(|collection| matches!(
                    collection.result(),
                    mech_engine::native_artifact::NativeCollectionResult::Histogram(_)
                ))
                .count(),
            1
        );
        assert_eq!(
            plan.iterations()
                .iter()
                .filter(|iteration| matches!(
                    iteration,
                    mech_engine::native_artifact::NativeControlLoop::Collection(generator)
                        if matches!(generator.source(), mech_engine::native_artifact::NativeCollectionSource::U64Range(_))
                ))
                .count(),
            1
        );
        assert!(
            plan.blocks()
                .iter()
                .all(|block| !matches!(block.action(), NativeIterationAction::LibraryCall { .. }))
        );
        let binding = jit
            .descriptor()
            .calls
            .iter()
            .flatten()
            .map(|call| call.binding_metadata_bytes())
            .max()
            .unwrap();
        binding + collection::State::metadata_bytes(jit.descriptor()).unwrap()
    }
    fn captured_roots(bytes: usize, count: u8) -> Vec<String> {
        (0..count)
            .map(|index| format!("{}{}", char::from(b'A' + index), "x".repeat(bytes - 1)))
            .collect()
    }
    fn captured_args(
        artifact: &mech_engine::ProgramArtifact,
        roots: &[String],
        memory: &ManagedMemoryBudget,
    ) -> Vec<Value> {
        artifact
            .inputs()
            .iter()
            .map(|input| {
                let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                assert_eq!(name.len(), 1, "single-letter captured fixture root");
                let index = usize::from(name.as_bytes()[0] - b'a');
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::String(roots[index].clone()),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut token = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                    .unwrap();
                value.into_memory_budget(&mut token).unwrap()
            })
            .collect()
    }
    fn row(rows: &[Row], demand: Demand) -> &Row {
        let selected = rows
            .iter()
            .filter(|row| row.visit == demand.visit)
            .collect::<Vec<_>>();
        assert_eq!(
            selected.len(),
            1,
            "actual stage must be entered exactly once: {:?}; rows={rows:?}",
            demand.visit
        );
        let row = selected[0];
        assert!(
            row.demand.is_some(),
            "actual search and stage preflight must be reached: {row:?}"
        );
        assert_eq!(
            row.demand.unwrap().stage,
            demand.stage,
            "an earlier boundary inside the helper does not qualify the requested stage"
        );
        row
    }
    fn unchanged(row: &Row, demand: Demand) {
        assert_eq!(
            row.after,
            Some((
                demand.visit.accepted_before,
                demand.distinct_before,
                demand.payload_before,
                demand.capacity_before
            ))
        );
        assert_eq!(
            row.memory_after, row.memory_before,
            "no future-output capacity credit on denial"
        );
        assert!(
            row.work_after >= row.work_before,
            "spent work is never refunded"
        );
    }
    fn required(demand: Demand, dimension: Dimension) -> u64 {
        match dimension {
            Dimension::TemporaryBytes => demand.temporary_required,
            Dimension::ComputeWork => demand.compute_required,
            Dimension::ClonedBytes => demand.cloned_required,
            Dimension::OutputBytes => demand.output_required,
            Dimension::OutputElements => demand.elements_required,
            Dimension::RetainedNodes => demand.nodes_required,
            Dimension::ComparisonWork => demand.comparison_required,
            _ => panic!("not an isolated boundary"),
        }
    }
    fn cap(caps: &mut NativeProgramExecutionLimits, dimension: Dimension, value: u64) {
        match dimension {
            Dimension::TemporaryBytes => caps.library.max_temporary_bytes = Some(value),
            Dimension::ComputeWork => caps.library.max_compute_work = Some(value),
            Dimension::ClonedBytes => caps.library.max_cloned_bytes = Some(value),
            Dimension::OutputBytes => caps.library.max_output_bytes = Some(value),
            Dimension::OutputElements => caps.library.max_output_elements = Some(value),
            Dimension::RetainedNodes => caps.library.max_retained_nodes = Some(value),
            Dimension::ComparisonWork => caps.library.max_comparison_work = Some(value),
            _ => panic!("not an isolated boundary"),
        }
    }
    #[test]
    fn actual_histogram_growth_and_finish_exact_local_caps_precede_mutation_and_recover() {
        // The generated body selects existing managed String roots, not a
        // String element/concatenation factory with an earlier large envelope.
        // Four tiny keys establish capacity four; the last large key forces the
        // actual 4->8 vector growth. Its copied key and untracked captured owner
        // coexist, so 2*large alone exceeds every fixed binding prefix. Keeping
        // the other keys small leaves ample unchanged compute/comparison room.
        let source = "+> map\nmap/histogram([(index? | 1u64 => a<string> | 2u64 => b<string> | 3u64 => c<string> | 4u64 => d<string> | * => e<string>) | index <- 1u64..=5u64])\n";
        for artifact in artifacts_source(source) {
            let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let binding_peak = captured_binding_peak(&jit);
            let large = usize::try_from(binding_peak.div_ceil(2) + 1024).unwrap();
            let mut roots = captured_roots(8, 4);
            roots.push(format!("E{}", "x".repeat(large - 1)));
            let args = captured_args(&artifact, &roots, &memory);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            let prior = snapshot(&artifact, &outputs);
            let stable = memory.used_bytes();
            let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            outcome.unwrap();
            let growth = baseline
                .iter()
                .filter_map(|row| row.demand)
                .filter(|d| d.stage == Stage::Growth)
                .max_by_key(|d| d.visit.accepted_before)
                .unwrap();
            let finish = baseline
                .iter()
                .filter_map(|row| row.demand)
                .find(|d| d.stage == Stage::Finish)
                .unwrap();
            assert!(
                growth.distinct_before == 4
                    && growth.capacity_before == 4
                    && growth.old_backing_bytes > 0
                    && growth.current_yield_bytes > 0
            );
            assert_eq!(growth.current_yield_bytes, growth.untracked_yield_bytes);
            assert!(growth.current_yield_bytes >= large as u64);
            assert!(growth.temporary_required > binding_peak);
            assert_eq!(
                growth.reservation_bytes,
                large as u64 + 8 * collection::State::test_histogram_entry_bytes()
            );
            assert!(
                growth.temporary_required
                    >= growth.old_backing_bytes
                        + growth.reservation_bytes
                        + growth.current_yield_bytes
            );
            assert_eq!(growth.visit.accepted_before, growth.distinct_before);
            assert_eq!(
                finish.visit.accepted_before,
                growth.visit.accepted_before + 1
            );
            assert_eq!(finish.output_required, growth.output_required);
            for (demand, dimensions) in [
                (
                    growth,
                    &[
                        Dimension::TemporaryBytes,
                        Dimension::ComputeWork,
                        Dimension::ClonedBytes,
                        Dimension::OutputBytes,
                        Dimension::OutputElements,
                        Dimension::RetainedNodes,
                    ][..],
                ),
                (
                    finish,
                    &[
                        Dimension::TemporaryBytes,
                        Dimension::ComputeWork,
                        Dimension::ClonedBytes,
                        Dimension::ComparisonWork,
                    ][..],
                ),
            ] {
                for &dimension in dimensions {
                    let bound = required(demand, dimension);
                    assert!(bound > 0);
                    let mut exact = caps();
                    cap(&mut exact, dimension, bound);
                    let (outcome, rows) = observed(&jit, &args, &mut outputs, exact, &memory, None);
                    assert!(
                        rows.iter().any(|row| row.visit == demand.visit),
                        "exact {dimension:?}={bound} did not reach {:?}: {outcome:?}",
                        demand.visit
                    );
                    assert_eq!(
                        row(&rows, demand).status,
                        STATUS_OK,
                        "exact {dimension:?}; outcome={outcome:?}"
                    );
                    if let Err(error) = outcome {
                        assert!(
                            matches!(error,NativeProgramExecutionError::Collection{error:ResidentKernelError::ResourceLimit{dimension:actual,..},..}if actual==dimension),
                            "{error:?}"
                        );
                    }
                    assert_eq!(snapshot(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                    let mut low = caps();
                    cap(&mut low, dimension, bound - 1);
                    let (outcome, rows) = observed(&jit, &args, &mut outputs, low, &memory, None);
                    assert!(
                        rows.iter().any(|row| row.visit == demand.visit),
                        "one-short {dimension:?}={bound}-1 did not reach {:?}: {outcome:?}",
                        demand.visit
                    );
                    let denied = row(&rows, demand);
                    assert_ne!(denied.status, STATUS_OK);
                    let error = outcome.unwrap_err();
                    assert!(
                        matches!(error,NativeProgramExecutionError::Collection{origin,error:ResidentKernelError::ResourceLimit{phase:ResidentResourcePhase::Admission,dimension:actual,required,limit}}if origin==demand.origin&&actual==dimension&&required==bound&&limit==bound-1),
                        "{error:?}"
                    );
                    unchanged(denied, demand);
                    assert_eq!(snapshot(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                    jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                        .unwrap();
                    assert_eq!(snapshot(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                }
            }
            // Output/nodes/elements at Finish already bind the last accepted key;
            // a globally one-short cap correctly denies the preceding growth.
            // The separate finite cumulative comparison cap is isolated at Finish,
            // after the real binary search prefix fits unchanged.
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    #[test]
    fn actual_histogram_growth_and_finish_global_coexistence_exact_one_short_and_recovery() {
        for artifact in artifacts() {
            let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let args = args(&artifact, jit.descriptor().scalar_bytes, &memory);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            let prior = snapshot(&artifact, &outputs);
            let stable = memory.used_bytes();
            let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            outcome.unwrap();
            let growth = baseline
                .iter()
                .filter_map(|row| row.demand)
                .filter(|d| d.stage == Stage::Growth)
                .max_by_key(|d| d.visit.accepted_before)
                .unwrap();
            let finish = baseline
                .iter()
                .filter_map(|row| row.demand)
                .find(|d| d.stage == Stage::Finish)
                .unwrap();
            for demand in [growth, finish] {
                assert!(demand.reservation_bytes > 0 && demand.compute_debit > 0);
                let (outcome, rows) = observed(
                    &jit,
                    &args,
                    &mut outputs,
                    caps(),
                    &memory,
                    Some(Pressure {
                        demand,
                        short: false,
                    }),
                );
                outcome.unwrap();
                let exact = row(&rows, demand);
                assert_eq!(exact.status, STATUS_OK);
                assert_eq!(exact.global_remaining, Some(demand.reservation_bytes));
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                let (outcome, rows) = observed(
                    &jit,
                    &args,
                    &mut outputs,
                    caps(),
                    &memory,
                    Some(Pressure {
                        demand,
                        short: true,
                    }),
                );
                let denied = row(&rows, demand);
                assert_ne!(denied.status, STATUS_OK);
                assert_ne!(
                    denied.status,
                    u64::MAX,
                    "pressure installation must succeed"
                );
                assert_eq!(denied.global_remaining, Some(demand.reservation_bytes - 1));
                let error = outcome.unwrap_err();
                assert!(
                    matches!(error,NativeProgramExecutionError::Base(NativeExecutionError::Memory{error:mech_core::MemoryRuntimeError::BudgetExceeded{requested,limit,..}})if requested==memory.limit_bytes()+1&&limit==memory.limit_bytes()),
                    "{error:?}"
                );
                unchanged(denied, demand);
                assert!(denied.work_after - denied.work_before >= demand.compute_debit);
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                    .unwrap();
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
            }
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }

    // INLINE inside histogram_observation_tests, after its shared helpers.
    #[test]
    fn actual_histogram_duplicate_body_yield_exact_temporary_peak_and_one_short_recover() {
        // Generated matches copy pre-admitted captured String roots. No per-yield
        // String-element/concatenation admission can hide the actual last yield.
        // Four large keys, four tiny keys and then a large duplicate make that
        // duplicate's untracked owner coexist with strictly more key/vector
        // backing than any earlier new-key allocation. Preserve the existing
        // fixture policy unchanged; only its source/argument geometry changes.
        let source = "+> map\nmap/histogram([(index? | 1u64 => a<string> | 2u64 => b<string> | 3u64 => c<string> | 4u64 => d<string> | 5u64 => e<string> | 6u64 => f<string> | 7u64 => g<string> | 8u64 => h<string> | * => c) | index <- 1u64..=9u64])\n";
        for artifact in artifacts_source(source) {
            let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let mut policy = caps();
            policy.library.max_compute_work = Some(1_000_000_000);
            policy.library.max_comparison_work = Some(1_000_000_000);
            let binding_peak = captured_binding_peak(&jit);
            let large = usize::try_from(binding_peak.div_ceil(5) + 1024).unwrap();
            let mut roots = captured_roots(large, 4);
            roots.extend((0..4).map(|index| format!("s{index}")));
            let args = captured_args(&artifact, &roots, &memory);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, policy, &memory)
                .unwrap();
            let prior = snapshot(&artifact, &outputs);
            let stable = memory.used_bytes();
            let (outcome, rows) = observed(&jit, &args, &mut outputs, policy, &memory, None);
            outcome.unwrap();
            let duplicate = rows
                .iter()
                .filter_map(|row| row.demand)
                .find(|demand| demand.stage == Stage::Yield && demand.visit.accepted_before == 8)
                .expect("actual last duplicate body yield preflight");
            assert_eq!(duplicate.distinct_before, 8);
            assert_eq!(duplicate.reservation_bytes, 0);
            assert!(
                duplicate.current_yield_bytes >= large as u64
                    && duplicate.untracked_yield_bytes == duplicate.current_yield_bytes
            );
            assert!(duplicate.temporary_required > binding_peak);
            assert_eq!(
                duplicate.temporary_required,
                duplicate.live_bytes + duplicate.untracked_yield_bytes
            );
            assert_eq!(
                duplicate.compute_debit, 0,
                "duplicate yield admission invents no append/copy/growth"
            );
            // Every earlier recorded growth must fit strictly below this local cap;
            // row(stage) below also rejects an earlier source/leaf denial loophole.
            assert!(
                rows.iter()
                    .filter_map(|row| row.demand)
                    .filter(|d| d.stage == Stage::Growth)
                    .all(|d| d.temporary_required < duplicate.temporary_required)
            );
            let mut exact = policy;
            exact.library.max_temporary_bytes = Some(duplicate.temporary_required);
            let (outcome, rows) = observed(&jit, &args, &mut outputs, exact, &memory, None);
            assert!(
                rows.iter().any(|row| row.visit == duplicate.visit),
                "exact duplicate Temporary={} did not reach {:?}: {outcome:?}",
                duplicate.temporary_required,
                duplicate.visit
            );
            assert_eq!(row(&rows, duplicate).status, STATUS_OK, "{outcome:?}");
            if let Err(error) = outcome {
                assert!(
                    matches!(
                        error,
                        NativeProgramExecutionError::Collection {
                            error: ResidentKernelError::ResourceLimit {
                                phase: ResidentResourcePhase::Admission,
                                dimension: Dimension::TemporaryBytes,
                                ..
                            },
                            ..
                        }
                    ),
                    "{error:?}"
                );
            }
            assert_eq!(snapshot(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
            let mut low = policy;
            low.library.max_temporary_bytes = Some(duplicate.temporary_required - 1);
            let (outcome, rows) = observed(&jit, &args, &mut outputs, low, &memory, None);
            assert!(
                rows.iter().any(|row| row.visit == duplicate.visit),
                "one-short duplicate Temporary={}-1 did not reach {:?}: {outcome:?}",
                duplicate.temporary_required,
                duplicate.visit
            );
            let denied = row(&rows, duplicate);
            assert_ne!(denied.status, STATUS_OK);
            let error = outcome.unwrap_err();
            assert!(
                matches!(error,NativeProgramExecutionError::Collection{origin,error:ResidentKernelError::ResourceLimit{phase:ResidentResourcePhase::Admission,dimension:Dimension::TemporaryBytes,required,limit}}if origin==duplicate.origin&&required==duplicate.temporary_required&&limit==required-1),
                "{error:?}"
            );
            unchanged(denied, duplicate);
            assert_eq!(snapshot(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
            jit.evaluate_into(&args, &mut outputs, policy, &memory)
                .unwrap();
            assert_eq!(snapshot(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod text_join_observation_tests {
    //! Observe actual emitted append/finalize calls without replacing their work.
    //! The observer never changes a policy, payload, visit, operation, or argument.
    use super::*;
    use collection::{TextJoinDemand, TextJoinStage};
    use mech_core::{
        MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase,
    };
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Target {
        collection: u32,
        stage: TextJoinStage,
        accepted_before: u64,
    }
    impl Target {
        fn of(demand: &TextJoinDemand) -> Self {
            Self {
                collection: demand.collection,
                stage: demand.stage,
                accepted_before: demand.accepted_before,
            }
        }
    }
    #[derive(Clone, Copy)]
    struct Pressure {
        target: Target,
        short: bool,
    }
    #[derive(Clone, Copy, Debug)]
    struct Observation {
        before: TextJoinDemand,
        after_storage: Option<(u64, u64, u64)>,
        memory_before: u64,
        memory_after: u64,
        work_before: u64,
        work_after: u64,
        global_remaining: Option<u64>,
        status: u64,
    }
    thread_local! {
        static OBSERVATIONS: std::cell::RefCell<Vec<Observation>> = const { std::cell::RefCell::new(Vec::new()) };
        static PRESSURE: std::cell::Cell<Option<Pressure>> = const { std::cell::Cell::new(None) };
    }

    // Same checked callback, with a shared header read before/after its exclusive
    // borrow. In particular no assertion may unwind through this C callback.
    unsafe extern "C" fn observe(
        context: *mut c_void,
        site: u64,
        operation: u64,
        first: u64,
        second: u64,
        third: u64,
        result: *mut u64,
    ) -> u64 {
        let before = (|| {
            // SAFETY: execute retains this exact context for the synchronous JIT
            // entry. This shared borrow ends before calling the real helper.
            let ctx = unsafe { &*context.cast::<Context<'_, '_>>() };
            let state = ctx.collections.as_ref()?;
            let demand = state
                .test_text_join_demand(ctx, usize::try_from(site).ok()?)
                .ok()??;
            Some((demand, ctx.library.memory_budget(), ctx.control.work.used()))
        })();
        let mut pressure_failure = false;
        let mut ballast = None;
        let mut global_remaining = None;
        if let Some((demand, memory, _)) = &before {
            if let Some(pressure) = PRESSURE.with(std::cell::Cell::get)
                && pressure.target == Target::of(demand)
            {
                let remaining = demand
                    .reservation_bytes
                    .saturating_sub(u64::from(pressure.short));
                let amount = memory
                    .limit_bytes()
                    .checked_sub(memory.used_bytes())
                    .and_then(|available| available.checked_sub(remaining));
                match amount {
                    Some(amount) => match memory.reserve_capacity(amount) {
                        Ok(reservation) => {
                            ballast = Some(reservation);
                            global_remaining = Some(remaining);
                        }
                        Err(_) => pressure_failure = true,
                    },
                    None => pressure_failure = true,
                }
            }
        }
        // Pressure installation is test infrastructure only. A failure is recorded
        // after the same real callback; it never chooses a replacement computation.
        let memory_before = before.as_ref().map(|(_, memory, _)| memory.used_bytes());
        // SAFETY: context, site, operation, all words and result are identical to
        // the generated call; the production helper is the only authority.
        let status =
            unsafe { super::helper(context, site, operation, first, second, third, result) };
        if let Some((demand, memory, work_before)) = before {
            // SAFETY: helper returned and its exclusive borrow has ended.
            let ctx = unsafe { &*context.cast::<Context<'_, '_>>() };
            let after_storage = ctx
                .collections
                .as_ref()
                .and_then(|state| state.test_text_join_storage(demand.collection));
            let row = Observation {
                before: demand,
                after_storage,
                memory_before: memory_before.unwrap_or(0),
                memory_after: memory.used_bytes(),
                work_before,
                work_after: ctx.control.work.used(),
                global_remaining,
                status: if pressure_failure { u64::MAX } else { status },
            };
            OBSERVATIONS.with(|rows| rows.borrow_mut().push(row));
        }
        // Exact global pressure exists ONLY around the selected real stage. Later
        // arena/publication phases get the original unchanged account capacity.
        drop(ballast);
        status
    }

    fn observed(
        jit: &super::super::JitProgramArtifact,
        args: &[Value],
        outputs: &mut [Option<Value>],
        caps: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
        pressure: Option<Pressure>,
    ) -> (
        Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
        Vec<Observation>,
    ) {
        OBSERVATIONS.with(|rows| rows.borrow_mut().clear());
        PRESSURE.with(|slot| slot.set(pressure));
        let entry = jit.test_checked_entry();
        let outcome = execute(
            jit.descriptor(),
            |raw| {
                // SAFETY: the JIT owner remains alive. Replace only the transport
                // observer, invoke the same actual entry, and restore before the
                // production immutable-frame check.
                let original = unsafe { (*raw).helper };
                unsafe { (*raw).helper = observe };
                let status = unsafe { entry(raw) };
                unsafe { (*raw).helper = original };
                status
            },
            args,
            outputs,
            caps,
            memory,
        );
        PRESSURE.with(|slot| slot.set(None));
        let rows = OBSERVATIONS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
        (outcome, rows)
    }

    fn artifacts() -> [mech_engine::ProgramArtifact; 2] {
        let source = "+> text\ntext/join([part | part <- source<[string]>], separator<string>)\n";
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x4a4f494e44454d), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let original = mech_engine::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        [original, decoded]
    }
    fn caps() -> NativeProgramExecutionLimits {
        let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        library.max_compute_work = Some(134_217_728);
        library.max_comparison_work = Some(1_000_000);
        library.max_temporary_bytes = Some(128 * 1024 * 1024);
        NativeProgramExecutionLimits {
            execution: NativeExecutionLimits {
                max_work: 1_000_000_000,
                ..NativeExecutionLimits::default()
            },
            library,
        }
    }
    fn args(artifact: &mech_engine::ProgramArtifact, memory: &ManagedMemoryBudget) -> Vec<Value> {
        let text = "a".repeat(96 * 1024);
        artifact
            .inputs()
            .iter()
            .map(|input| {
                let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                let data = match name.as_str() {
                    "source" => ValueDataDraft::Matrix(
                        // Four nonempty yields force the last geometric growth
                        // above the preceding element's construction envelope.
                        // With three, an exact growth cap can deny that earlier
                        // helper and never exercise the intended boundary.
                        (0..4)
                            .map(|_| ValueDataDraft::String(text.clone()))
                            .collect(),
                    ),
                    "separator" => ValueDataDraft::String("|é\0".repeat(32)),
                    _ => panic!("unexpected input {name}"),
                };
                let shape_values = match &data {
                    ValueDataDraft::Matrix(values) => mech_core::shape_for_resolved_extents(
                        artifact.schemas().get(input.schema).unwrap(),
                        &[1, values.len() as u64],
                    )
                    .unwrap()
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice(),
                    _ => Box::new([]),
                };
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values,
                    data,
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                    .unwrap();
                value.into_memory_budget(&mut reservation).unwrap()
            })
            .collect()
    }
    fn snapshot(
        artifact: &mech_engine::ProgramArtifact,
        outputs: &[Option<Value>],
    ) -> Vec<Box<[u8]>> {
        outputs
            .iter()
            .map(|value| {
                value
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap()
            })
            .collect()
    }
    fn row(rows: &[Observation], target: Target) -> &Observation {
        let selected = rows
            .iter()
            .filter(|row| Target::of(&row.before) == target)
            .collect::<Vec<_>>();
        assert_eq!(
            selected.len(),
            1,
            "selected actual stage must be reached exactly once: {target:?}"
        );
        selected[0]
    }
    fn unchanged_payload(row: &Observation) {
        let after = row
            .after_storage
            .expect("failed stage keeps its unpublished builder until unwind");
        assert_eq!(after.0, row.before.accepted_before);
        assert_eq!(after.1, row.before.payload_before);
        assert_eq!(after.2, row.before.capacity_before);
        assert_eq!(
            row.memory_after, row.memory_before,
            "denied real reservation grants no future-output credit"
        );
        assert!(
            row.work_after >= row.work_before,
            "spent work is never refunded"
        );
    }
    fn required(demand: &TextJoinDemand, dimension: Dimension) -> u64 {
        match dimension {
            Dimension::TemporaryBytes => demand.temporary_required,
            Dimension::ComputeWork => demand.compute_required,
            Dimension::ClonedBytes => demand.cloned_required,
            Dimension::OutputBytes => demand.output_required,
            _ => panic!("unsupported isolated boundary"),
        }
    }
    fn set_cap(caps: &mut NativeProgramExecutionLimits, dimension: Dimension, amount: u64) {
        match dimension {
            Dimension::TemporaryBytes => caps.library.max_temporary_bytes = Some(amount),
            Dimension::ComputeWork => caps.library.max_compute_work = Some(amount),
            Dimension::ClonedBytes => caps.library.max_cloned_bytes = Some(amount),
            Dimension::OutputBytes => caps.library.max_output_bytes = Some(amount),
            _ => panic!("unsupported isolated boundary"),
        }
    }

    #[test]
    fn actual_join_growth_and_finish_exact_local_caps_precede_mutation_and_recover() {
        for artifact in artifacts() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let args = args(&artifact, &memory);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            let prior = snapshot(&artifact, &outputs);
            let stable = memory.used_bytes();
            let (accepted, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            accepted.unwrap();
            let growth = baseline
                .iter()
                .filter(|row| row.before.stage == TextJoinStage::Growth)
                .max_by_key(|row| row.before.accepted_before)
                .expect("actual growth")
                .before;
            let finish = baseline
                .iter()
                .find(|row| row.before.stage == TextJoinStage::Finish)
                .expect("actual consumer finalization")
                .before;
            assert_eq!(growth.accepted_before, 3);
            assert!(
                growth.old_backing_bytes > 0
                    && growth.current_yield_bytes > 0
                    && growth.separator_bytes > 0
            );
            assert!(
                growth.temporary_required
                    >= growth.old_backing_bytes
                        + growth.reservation_bytes
                        + growth.current_yield_bytes
                        + growth.separator_bytes
            );
            assert_eq!(finish.accepted_before, 4);
            assert_eq!(finish.output_required, growth.output_required);
            for (demand, dimensions) in [
                (
                    growth,
                    &[
                        Dimension::TemporaryBytes,
                        Dimension::ComputeWork,
                        Dimension::ClonedBytes,
                        Dimension::OutputBytes,
                    ][..],
                ),
                (
                    finish,
                    &[
                        Dimension::TemporaryBytes,
                        Dimension::ComputeWork,
                        Dimension::ClonedBytes,
                    ][..],
                ),
            ] {
                let target = Target::of(&demand);
                for &dimension in dimensions {
                    let bound = required(&demand, dimension);
                    assert!(bound > 0);
                    let mut exact = caps();
                    set_cap(&mut exact, dimension, bound);
                    let (outcome, rows) = observed(&jit, &args, &mut outputs, exact, &memory, None);
                    assert!(
                        rows.iter().any(|row| Target::of(&row.before) == target),
                        "exact {dimension:?} cap {bound} must reach {target:?}; outcome={outcome:?}; rows={rows:?}"
                    );
                    assert_eq!(
                        row(&rows, target).status,
                        STATUS_OK,
                        "{dimension:?} exact stage must succeed"
                    );
                    // A later phase may need a higher bound than this targeted
                    // growth. Never report whole-invocation acceptance from that.
                    if let Err(error) = outcome {
                        assert!(
                            matches!(error, NativeProgramExecutionError::Collection {
                            error: ResidentKernelError::ResourceLimit { dimension: actual, .. }, ..
                        } if actual == dimension),
                            "{error:?}"
                        );
                    }
                    assert_eq!(snapshot(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                    let mut low = caps();
                    set_cap(&mut low, dimension, bound - 1);
                    let (outcome, rows) = observed(&jit, &args, &mut outputs, low, &memory, None);
                    assert!(
                        rows.iter().any(|row| Target::of(&row.before) == target),
                        "one-short {dimension:?} cap {} must reach {target:?}; outcome={outcome:?}; rows={rows:?}",
                        bound - 1
                    );
                    let denied = row(&rows, target);
                    assert_ne!(denied.status, STATUS_OK);
                    let error = outcome.unwrap_err();
                    let origin =
                        if matches!(dimension, Dimension::ComputeWork | Dimension::ClonedBytes) {
                            demand.account_origin
                        } else {
                            demand.origin
                        };
                    assert!(
                        matches!(error, NativeProgramExecutionError::Collection {
                        origin: actual_origin,
                        error: ResidentKernelError::ResourceLimit { phase: ResidentResourcePhase::Admission,
                            dimension: actual, required, limit }, ..
                    } if actual_origin == origin && actual == dimension && required == bound && limit == bound - 1),
                        "{error:?}"
                    );
                    unchanged_payload(denied);
                    assert_eq!(snapshot(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                    jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                        .unwrap();
                    assert_eq!(snapshot(&artifact, &outputs), prior);
                    assert_eq!(memory.used_bytes(), stable);
                }
            }
            // Finish OutputBytes is already required by the preceding last append;
            // a one-short policy would correctly deny there, not at Finish. Nodes
            // are likewise needed at Begin. Do not mutate caps inside the observer
            // or pretend those earlier denials qualify a later-stage boundary.
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }

    #[test]
    fn actual_join_growth_and_finish_global_coexistence_boundary_has_no_credit_or_refund() {
        for artifact in artifacts() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit = super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let args = args(&artifact, &memory);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            let prior = snapshot(&artifact, &outputs);
            let stable = memory.used_bytes();
            let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
            outcome.unwrap();
            let growth = baseline
                .iter()
                .filter(|row| row.before.stage == TextJoinStage::Growth)
                .max_by_key(|row| row.before.accepted_before)
                .unwrap()
                .before;
            let finish = baseline
                .iter()
                .find(|row| row.before.stage == TextJoinStage::Finish)
                .unwrap()
                .before;
            for demand in [growth, finish] {
                let target = Target::of(&demand);
                assert!(demand.reservation_bytes > 0 && demand.compute_debit > 0);
                let (exact, rows) = observed(
                    &jit,
                    &args,
                    &mut outputs,
                    caps(),
                    &memory,
                    Some(Pressure {
                        target,
                        short: false,
                    }),
                );
                exact.unwrap();
                let exact = row(&rows, target);
                assert_eq!(exact.status, STATUS_OK);
                assert_eq!(exact.global_remaining, Some(demand.reservation_bytes));
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                let (denied, rows) = observed(
                    &jit,
                    &args,
                    &mut outputs,
                    caps(),
                    &memory,
                    Some(Pressure {
                        target,
                        short: true,
                    }),
                );
                let denied_row = row(&rows, target);
                assert_ne!(denied_row.status, STATUS_OK);
                assert_ne!(
                    denied_row.status,
                    u64::MAX,
                    "test pressure installation must succeed"
                );
                assert_eq!(
                    denied_row.global_remaining,
                    Some(demand.reservation_bytes - 1)
                );
                let error = denied.unwrap_err();
                assert!(
                    matches!(error, NativeProgramExecutionError::Base(NativeExecutionError::Memory {
                    error: mech_core::MemoryRuntimeError::BudgetExceeded { requested, limit, .. }
                }) if requested == memory.limit_bytes() + 1 && limit == memory.limit_bytes()),
                    "{error:?}"
                );
                unchanged_payload(denied_row);
                assert!(
                    denied_row.work_after - denied_row.work_before >= demand.compute_debit,
                    "actual reservation denial follows its full own compute debit and never refunds it"
                );
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                    .unwrap();
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
            }
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
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
    pub(super) fn artifact_source(source: &str) -> mech_engine::ProgramArtifact {
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
    pub(super) fn limits() -> NativeProgramExecutionLimits {
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
    pub(super) fn input(
        artifact: &mech_engine::ProgramArtifact,
        memory: &ManagedMemoryBudget,
    ) -> Value {
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
                    // SAFETY: arguments remain within the retained ABI16 boundary;
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
    fn range_cursor_protocol_rejects_wrong_skipped_duplicate_and_malformed_preparation_atomically()
    {
        use mech_engine::native_artifact::{NativeCollectionSource, NativeControlLoop};
        let artifact = artifact_source("[i | i <- 0u64..count<u64>]\n");
        let jit =
            super::super::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        let descriptor = jit.descriptor();
        let prepare = descriptor
            .plan
            .iterations()
            .iter()
            .find_map(|data| match data {
                NativeControlLoop::Collection(data) => match data.source() {
                    NativeCollectionSource::U64Range(cursor) => Some(cursor.prepare()),
                    _ => None,
                },
                _ => None,
            })
            .expect("fixture really selects the source cursor");
        for mode in 0..6 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::U64(4),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut admission = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            let args = [value.into_memory_budget(&mut admission).unwrap()];
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            let prior = outputs[0]
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap();
            let before = memory.used_bytes();
            let error = execute(
                descriptor,
                |raw| {
                    // SAFETY: execute owns and retains this private frame and all
                    // callbacks; only the deliberately invalid sequence varies.
                    let frame = unsafe { &mut *raw };
                    if checkpoint_empty_activation(frame, descriptor) {
                        return super::super::program_abi::STATUS_ACTIVATED;
                    }
                    let mut site = descriptor.plan.activation_boundary().unwrap();
                    let next = |site: u32| {
                        let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
                            descriptor.plan.blocks()[site as usize].terminator()
                        else {
                            panic!("straight range setup")
                        };
                        next
                    };
                    loop {
                        assert_eq!(
                            unsafe { (frame.base.admit_next)(frame.base.context, u64::from(site)) },
                            STATUS_OK
                        );
                        if site == prepare {
                            break;
                        }
                        match descriptor.plan.blocks()[site as usize].action() {
                            NativeIterationAction::CollectionBegin { .. } => {
                                // Inline ranges are prepared inside a live
                                // builder. Consume its real required helper
                                // before testing a malformed preparation.
                                assert_eq!(
                                    unsafe {
                                        (frame.helper)(
                                            frame.base.context,
                                            u64::from(site),
                                            COLLECTION,
                                            0,
                                            0,
                                            0,
                                            frame.result,
                                        )
                                    },
                                    STATUS_OK
                                );
                            }
                            NativeIterationAction::Scalar(NativeMatchAction::None) => {}
                            other => panic!("unexpected range protocol setup action: {other:?}"),
                        }
                        site = next(site);
                    }
                    if mode == 3 {
                        return unsafe {
                            (frame.base.admit_next)(frame.base.context, u64::from(next(site)))
                        };
                    }
                    let called_site = if mode == 0 { site + 1 } else { site };
                    let operation = if mode == 1 { MOVE } else { COLLECTION };
                    let result = if mode == 2 {
                        frame.result.wrapping_add(1)
                    } else {
                        frame.result
                    };
                    let third = u64::from(mode == 4);
                    let status = unsafe {
                        (frame.helper)(
                            frame.base.context,
                            u64::from(called_site),
                            operation,
                            0,
                            4,
                            third,
                            result,
                        )
                    };
                    if mode != 5 {
                        return status;
                    }
                    assert_eq!(status, STATUS_OK);
                    unsafe {
                        (frame.helper)(
                            frame.base.context,
                            u64::from(site),
                            COLLECTION,
                            0,
                            4,
                            0,
                            frame.result,
                        )
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
            assert_eq!(memory.used_bytes(), before, "mode {mode}");
            assert_eq!(
                outputs[0]
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap(),
                prior
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
            drop((outputs, args, admission));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    #[derive(Clone, Copy, Debug)]
    struct RangeScratchObservation {
        origin: NativeMatchOrigin,
        live_bytes: u64,
        scratch_bytes: u64,
        binding_work: u64,
        memory_before: u64,
        memory_after: u64,
        work_before: u64,
        work_after: u64,
        status: u64,
    }
    thread_local! {
        static RANGE_SCRATCH_OBSERVATIONS: std::cell::RefCell<Vec<RangeScratchObservation>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    // Transparent test observation only: every call delegates unchanged to
    // the production helper. No generator, body, branch or value executes here.
    unsafe extern "C" fn observe_range_scratch(
        context: *mut c_void,
        site: u64,
        operation: u64,
        first: u64,
        second: u64,
        third: u64,
        result: *mut u64,
    ) -> u64 {
        let before = (|| {
            // SAFETY: the actual generated entry forwards execute's retained
            // private context. This shared borrow ends before helper mutates it.
            let ctx = unsafe { &*context.cast::<Context<'_, '_>>() };
            let site = usize::try_from(site).ok()?;
            let block = ctx.control.descriptor.plan.blocks().get(site)?;
            if !matches!(
                block.action(),
                NativeIterationAction::PrepareU64Range { .. }
            ) {
                return None;
            }
            let selected = ctx.control.descriptor.calls.get(site)?.as_ref()?;
            Some(RangeScratchObservation {
                origin: block.origin(),
                live_bytes: ctx.collections.as_ref()?.test_live_bytes(),
                scratch_bytes: selected.binding_metadata_bytes(),
                binding_work: selected.binding_work(),
                memory_before: ctx.library.memory_budget().used_bytes(),
                memory_after: 0,
                work_before: ctx.control.work.used(),
                work_after: 0,
                status: 0,
            })
        })();
        // SAFETY: identical checked context, site, operation, words and result
        // pointer; the production helper remains the only execution authority.
        let status =
            unsafe { super::helper(context, site, operation, first, second, third, result) };
        if let Some(mut observation) = before {
            // SAFETY: helper has returned and its exclusive borrow ended.
            let ctx = unsafe { &*context.cast::<Context<'_, '_>>() };
            observation.memory_after = ctx.library.memory_budget().used_bytes();
            observation.work_after = ctx.control.work.used();
            observation.status = status;
            RANGE_SCRATCH_OBSERVATIONS.with(|rows| rows.borrow_mut().push(observation));
        }
        status
    }
    fn execute_observed_range_scratch(
        jit: &super::super::JitProgramArtifact,
        args: &[Value],
        outputs: &mut [Option<Value>],
        caps: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
    ) -> (
        Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
        Vec<RangeScratchObservation>,
    ) {
        RANGE_SCRATCH_OBSERVATIONS.with(|rows| rows.borrow_mut().clear());
        let entry = jit.test_checked_entry();
        let outcome = execute(
            jit.descriptor(),
            |raw| {
                // SAFETY: this test retains the exact checked JIT owner. Replace
                // only the transport observer for this synchronous entry and
                // restore the immutable wire before execute verifies the frame.
                let original = unsafe { (*raw).helper };
                unsafe {
                    (*raw).helper = observe_range_scratch;
                }
                let status = unsafe { entry(raw) };
                unsafe {
                    (*raw).helper = original;
                }
                status
            },
            args,
            outputs,
            caps,
            memory,
        );
        let observations =
            RANGE_SCRATCH_OBSERVATIONS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
        (outcome, observations)
    }
    #[test]
    fn nested_range_scratch_exact_caps_precede_traversal_and_preserve_coexisting_builder_backing() {
        use mech_core::{
            MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase,
        };
        let artifact = artifact_source("[i + j | i <- 1u64..=outer<u64>, j <- 1u64..=i]\n");
        let jit =
            super::super::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let value = ValueDraft {
            schema: artifact.inputs()[0].schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::U64(3),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let mut admission = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let args = [value.into_memory_budget(&mut admission).unwrap()];
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let prior = outputs
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .canonical_snapshot_bytes(artifact.schemas())
            .unwrap();
        let stable = memory.used_bytes();
        let (baseline, rows) =
            execute_observed_range_scratch(&jit, &args, &mut outputs, limits(), &memory);
        baseline.unwrap();
        assert_eq!(
            rows.len(),
            4,
            "one outer source plus a fresh inner source for each of three visits"
        );
        let fixed_metadata = collection::State::metadata_bytes(jit.descriptor()).unwrap();
        assert!(
            rows.iter().any(|row| row.live_bytes > fixed_metadata),
            "inner preparation must coexist with already-retained builder drafts"
        );
        for row in &rows {
            assert_eq!(row.status, STATUS_OK);
            assert_eq!(
                row.memory_after, row.memory_before,
                "temporary scratch retires before the helper returns"
            );
            assert_eq!(
                row.work_after - row.work_before,
                row.binding_work,
                "the unchanged full debit precedes the traversal/allocation"
            );
        }
        let temporary = rows
            .iter()
            .map(|row| row.live_bytes.checked_add(row.scratch_bytes).unwrap())
            .max()
            .unwrap();
        let compute = rows.iter().map(|row| row.binding_work).max().unwrap();
        let global_peak = rows
            .iter()
            .map(|row| row.memory_before.checked_add(row.scratch_bytes).unwrap())
            .max()
            .unwrap();
        let mut exact = limits();
        exact.library.max_temporary_bytes = Some(temporary);
        exact.library.max_compute_work = Some(compute);
        let (accepted, exact_rows) =
            execute_observed_range_scratch(&jit, &args, &mut outputs, exact, &memory);
        accepted.unwrap();
        assert_eq!(exact_rows.len(), rows.len());
        assert_eq!(memory.used_bytes(), stable);
        let mut low = exact;
        low.library.max_temporary_bytes = Some(temporary - 1);
        let (denied, denied_rows) =
            execute_observed_range_scratch(&jit, &args, &mut outputs, low, &memory);
        let last = denied_rows
            .last()
            .expect("the denial must occur at actual range preparation");
        let error = denied.unwrap_err();
        assert!(
            matches!(error, NativeProgramExecutionError::Collection { origin,
            error: ResidentKernelError::ResourceLimit { phase: ResidentResourcePhase::Admission,
                dimension: Dimension::TemporaryBytes, required, limit }
        } if origin == last.origin && required == temporary && limit == temporary - 1),
            "{error:?}"
        );
        assert_eq!(
            last.work_after, last.work_before,
            "a denied cap does not start schema traversal"
        );
        assert_eq!(last.memory_after, last.memory_before);
        assert_eq!(memory.used_bytes(), stable);
        assert_eq!(
            outputs
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap(),
            prior
        );
        low = exact;
        low.library.max_compute_work = Some(compute - 1);
        let (denied, denied_rows) =
            execute_observed_range_scratch(&jit, &args, &mut outputs, low, &memory);
        let last = denied_rows.last().unwrap();
        let error = denied.unwrap_err();
        assert!(
            matches!(error, NativeProgramExecutionError::Collection { origin,
            error: ResidentKernelError::ResourceLimit { phase: ResidentResourcePhase::Admission,
                dimension: Dimension::ComputeWork, required, limit }
        } if origin == last.origin && required == compute && limit == compute - 1),
            "{error:?}"
        );
        assert_eq!(last.work_after, last.work_before);
        assert_eq!(last.memory_after, last.memory_before);
        assert_eq!(memory.used_bytes(), stable);
        // Account-wide coexistence is independently enforced, not replaced by
        // the local TemporaryBytes cap. Leave exactly the measured real peak.
        let mut ballast = memory
            .reserve_capacity(memory.limit_bytes() - global_peak)
            .unwrap();
        let (accepted, _) =
            execute_observed_range_scratch(&jit, &args, &mut outputs, exact, &memory);
        accepted.unwrap();
        ballast
            .resize_capacity(ballast.capacity_bytes() + 1)
            .unwrap();
        let (denied, denied_rows) =
            execute_observed_range_scratch(&jit, &args, &mut outputs, exact, &memory);
        let last = denied_rows
            .last()
            .expect("one byte less must deny the actual scratch reservation");
        let error = denied.unwrap_err();
        assert!(
            matches!(error, NativeProgramExecutionError::Base(NativeExecutionError::Memory {
            error: mech_core::MemoryRuntimeError::BudgetExceeded { requested, limit, .. }
        }) if requested == memory.limit_bytes() + 1 && limit == memory.limit_bytes()),
            "{error:?}"
        );
        assert_eq!(
            last.work_after - last.work_before,
            last.binding_work,
            "failed reservation never refunds the preceding work debit"
        );
        assert_eq!(
            last.memory_after, last.memory_before,
            "failed reservation changes no owner or Ready slot backing"
        );
        assert_eq!(
            outputs
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap(),
            prior
        );
        drop(ballast);
        assert_eq!(memory.used_bytes(), stable);
        let (recovered, _) =
            execute_observed_range_scratch(&jit, &args, &mut outputs, exact, &memory);
        recovered.unwrap();
        assert_eq!(
            outputs
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap(),
            prior
        );
        drop((args, outputs, admission));
        assert_eq!(memory.used_bytes(), 0);
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
                (32, 9u64),
                (32, 10u64),
                (32, 1u64),
                (32, 2u64),
                (32, 3u64),
                (32, 4u64),
                (16, 24u64),
                (16, 25u64),
                (24, 11u64),
                (32, 34u64),
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
    fn worker_abi_keeps_the_frozen_scalar_prefix_and_disjoint_helper_lanes() {
        assert_eq!(VERSION, 24);
        assert_eq!(super::super::program_abi::PHASE_CAPTURE_TAIL, 3);
        assert_eq!(super::super::program_abi::STATUS_INPUT_SCANNED, 7);
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
            core::mem::offset_of!(Frame, resume_site),
            core::mem::size_of::<NativeEntryFrame>() + 40
        );
        assert_eq!(
            core::mem::offset_of!(Frame, available_lines),
            core::mem::size_of::<NativeEntryFrame>() + 48
        );
        assert_eq!(
            core::mem::offset_of!(Frame, input_eof),
            core::mem::size_of::<NativeEntryFrame>() + 56
        );
        assert_eq!(
            core::mem::size_of::<Frame>(),
            core::mem::size_of::<NativeEntryFrame>() + 64
        );
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_structural_projection_tests.rs"]
mod structural_projection_tests;
