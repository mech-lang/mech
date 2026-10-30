//! Synchronous invocation borrows, dispatch and narrowly lent helper views.
use super::{NativeProgramExecutionError as Error, *};
use crate::native_artifact::iteration_runtime::ControlState;
use crate::native_artifact::program::runtime::maps::string::PublicationGeometry;
use crate::native_artifact::program::runtime::session::CallCacheLane;
use crate::native_artifact::{NativeValueHandle, NativeWorkMeter};
use mech_core::{MemoryBudgetLimits, Value};
use mech_engine::native_artifact::{NativeLibraryWorkBudget, NativeMatchValue};

mod buffer_frames;
mod numeric_segments;
mod numeric_writes;
mod readonly_numeric;
mod selected_calls;
mod unsigned_buffers;
mod word_reads;
mod word_updates;

/// A synchronous reborrow with a map-specific surface, not execution authority.
/// The enclosing invocation still owns phase validation and terminal handling.
pub(in crate::native_artifact::program::runtime) struct MapExecution<'borrow, 'state, 'visit, 'plan>
{
    invocation: &'borrow mut Invocation<'state, 'visit, 'plan>,
}
impl<'borrow, 'state, 'visit, 'plan> MapExecution<'borrow, 'state, 'visit, 'plan> {
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn new(
        invocation: &'borrow mut Invocation<'state, 'visit, 'plan>,
    ) -> Self {
        Self { invocation }
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn descriptor(
        &self,
    ) -> &'plan NativeProgramDescriptor {
        self.invocation.descriptor
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn control(&self) -> &ControlState {
        &self.invocation.state.control
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn work(&mut self) -> &mut NativeWorkMeter {
        &mut self.invocation.state.control.work
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn library(&self) -> &NativeLibraryWorkBudget {
        &self.invocation.state.library
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn limits(&self) -> MemoryBudgetLimits {
        self.invocation.limits
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn consumed(&self) -> bool {
        self.invocation.state.consumed
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn stats(&mut self) -> &mut RuntimeStats {
        &mut self.invocation.state.stats
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arguments(&self) -> *mut u64 {
        self.invocation.arguments
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn argument_capacity(&self) -> usize {
        self.invocation.argument_capacity
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn result_lane(&self) -> *mut u64 {
        self.invocation.result
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn caches(&self) -> &CallCacheLane<'plan> {
        &self.invocation.state.call_caches
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn caches_and_library(
        &mut self,
    ) -> (&mut CallCacheLane<'plan>, &NativeLibraryWorkBudget) {
        (
            &mut self.invocation.state.call_caches,
            &self.invocation.state.library,
        )
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_copy_current(
        &mut self,
        slot: u32,
    ) -> Result<Value, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.copy_current(slot, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_current_handle(
        &mut self,
        slot: u32,
    ) -> Result<NativeValueHandle, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.current_handle(slot, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_get(
        &mut self,
        handle: NativeValueHandle,
    ) -> Result<&Value, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.get(handle, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_put(
        &mut self,
        slot: u32,
        value: &Value,
    ) -> Result<NativeValueHandle, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.put(slot, value, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn scalar(
        &mut self,
        value: NativeMatchValue,
        word: u64,
    ) -> Result<Value, Error> {
        self.invocation.scalar(value, word)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn retained_caller_geometry(
        &mut self,
        site: usize,
    ) -> Result<Option<PublicationGeometry>, Error> {
        self.invocation.retained_caller_geometry(site)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn retained_caller_fixed_geometry(
        &mut self,
        site: usize,
    ) -> Result<Option<u64>, Error> {
        self.invocation.retained_caller_fixed_geometry(site)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn retained_caller_dynamic_nodes(
        &self,
        site: usize,
    ) -> Result<u64, Error> {
        self.invocation.retained_caller_dynamic_nodes(site)
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(in crate::native_artifact::program::runtime) fn retained_caller_fixed_inventory(
        &mut self,
    ) -> Result<Option<u64>, Error> {
        self.invocation.retained_caller_fixed_inventory()
    }
}

pub(in crate::native_artifact::program::runtime) struct Invocation<'state, 'visit, 'plan> {
    pub(in crate::native_artifact::program::runtime) state: &'state mut ExecutionState<'plan>,
    pub(in crate::native_artifact::program::runtime) descriptor: &'plan NativeProgramDescriptor,
    pub(in crate::native_artifact::program::runtime) retirements: &'visit [u64],
    pub(in crate::native_artifact::program::runtime) progress: &'visit mut [u64],
    pub(in crate::native_artifact::program::runtime) borrowed_strings: Option<borrowed::State>,
    pub(in crate::native_artifact::program::runtime) closed_tail:
        Option<crate::native_artifact::program::runtime::invocation::tail::State>,
    pub(in crate::native_artifact::program::runtime) selected_join:
        Option<crate::native_artifact::program::runtime::invocation::joins::State>,
    pub(in crate::native_artifact::program::runtime) limits: MemoryBudgetLimits,
    pub(in crate::native_artifact::program::runtime) result: *mut u64,
    pub(in crate::native_artifact::program::runtime) arguments: *mut u64,
    pub(in crate::native_artifact::program::runtime) argument_capacity: usize,
    pub(in crate::native_artifact::program::runtime) error: Option<NativeProgramExecutionError>,
    pub(in crate::native_artifact::program::runtime) activation_phase: bool,
    pub(in crate::native_artifact::program::runtime) execution_limits: NativeExecutionLimits,
    pub(in crate::native_artifact::program::runtime) worker: Option<parallel::Worker>,
    pub(in crate::native_artifact::program::runtime) parallel_batch: Option<parallel::Batch>,
    pub(in crate::native_artifact::program::runtime) window_region: Option<(u32, u64)>,
    pub(in crate::native_artifact::program::runtime) window_hashes: Option<
        Box<crate::native_artifact::program::runtime::invocation::window::hashes::WindowHashes>,
    >,
    pub(in crate::native_artifact::program::runtime) prefix: Option<prefix::State>,
    // Pins UTF-8 backing for this synchronous entry only.
    pub(in crate::native_artifact::program::runtime) line_window:
        Option<&'visit mech_engine::native_artifact::NativeTextLineWindow<'visit>>,
}
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn control(
        &mut self,
    ) -> Control<'_, '_, NativeProgramDescriptor> {
        Control {
            state: &mut self.state.control,
            descriptor: self.descriptor,
            progress: self.progress,
            error: None,
        }
    }
}

impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn write_borrowed_word(
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
    pub(in crate::native_artifact::program::runtime) fn read_borrowed_word(
        &self,
        index: usize,
    ) -> Result<u64, NativeProgramExecutionError> {
        if self.arguments.is_null() || index >= self.argument_capacity {
            return Err(invalid("native reconciliation word outside admitted lane"));
        }
        // SAFETY: the owning session registers this exact admitted lane. The
        // generated entry is suspended and no Rust reference crosses execution.
        Ok(unsafe { *self.arguments.add(index) })
    }
    pub(in crate::native_artifact::program::runtime) fn write_window_arguments(
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
    pub(in crate::native_artifact::program::runtime) fn needs_move(
        &self,
        site: usize,
    ) -> Option<(NativeMatchValue, NativeMatchValue)> {
        if self.borrowed_action(site).is_some() || self.retained_native_action(site) {
            return None;
        }
        if self
            .descriptor
            .metadata()
            .scalar_aggregates
            .action(site)
            .is_some()
        {
            return None;
        }
        let plan = &self.descriptor.metadata().plan;
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
    pub(in crate::native_artifact::program::runtime) fn action_required(
        &self,
        site: usize,
    ) -> bool {
        if self.borrowed_action(site).is_some() || self.retained_native_action(site) {
            return false;
        }
        if self
            .descriptor
            .metadata()
            .scalar_aggregates
            .action(site)
            .is_some()
        {
            return false;
        }
        matches!(
            self.descriptor.metadata().plan.blocks()[site].action(),
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
            || matches!(self.descriptor.metadata().plan.blocks()[site].action(),
                NativeIterationAction::Initialize { iteration }
                if matches!(self.descriptor.metadata().plan.iterations()[*iteration as usize], mech_engine::native_artifact::NativeControlLoop::Collection(_)))
    }
    pub(in crate::native_artifact::program::runtime) fn scalar(
        &mut self,
        value: NativeMatchValue,
        word: u64,
    ) -> Result<Value, NativeProgramExecutionError> {
        let descriptor = self.descriptor;
        let ty = &descriptor.metadata().plan.values()[value.get() as usize];
        let data = match ty.scalar_kind() {
            Some(NativeScalarKind::Bool) if word <= 1 => ValueDataDraft::Bool(word != 0),
            Some(NativeScalarKind::U8) => ValueDataDraft::U8(
                u8::try_from(word).map_err(|_| invalid("narrow scalar word exceeds its type"))?,
            ),
            Some(NativeScalarKind::U16) => ValueDataDraft::U16(
                u16::try_from(word).map_err(|_| invalid("narrow scalar word exceeds its type"))?,
            ),
            Some(NativeScalarKind::U32) => ValueDataDraft::U32(
                u32::try_from(word).map_err(|_| invalid("narrow scalar word exceeds its type"))?,
            ),
            Some(NativeScalarKind::U64) => ValueDataDraft::U64(word),
            Some(NativeScalarKind::I64) => ValueDataDraft::I64(word as i64),
            Some(NativeScalarKind::F64) => ValueDataDraft::F64(F64Bits::from_bits(word)),
            _ => return Err(invalid("invalid scalar helper operand")),
        };
        // Covers construction, validation, ownership import AND retirement on
        // any later failed insertion. No allocation happens before admission.
        let work = descriptor
            .metadata()
            .scalar_bytes
            .checked_mul(8)
            .and_then(|n| n.checked_add(128))
            .ok_or_else(|| invalid("scalar setup work overflow"))?;
        self.state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, work)?;
        let memory = self.state.library.memory_budget();
        let mut reservation = memory.reserve_capacity(descriptor.metadata().scalar_bytes)?;
        ValueDraft {
            schema: ty.schema(),
            shape_values: Box::new([]),
            data,
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &descriptor.metadata().schemas,
        ))
        .map_err(NativeProgramExecutionError::Snapshot)?
        .into_memory_budget(&mut reservation)
        .map_err(Into::into)
    }
    pub(in crate::native_artifact::program::runtime) fn helper(
        &mut self,
        site: u64,
        operation: u64,
        args: [u64; 3],
    ) -> Result<Option<u64>, NativeProgramExecutionError> {
        let tag = GlobalHelper::decode(operation);
        if self
            .prefix
            .as_ref()
            .is_some_and(|prefix| prefix.paused.is_some())
        {
            return Err(invalid("helper after native prefix checkpoint"));
        }
        if tag == GlobalHelper::ClosedTailEnd {
            let site = usize::try_from(site).map_err(|_| invalid("closed Tail site overflow"))?;
            return self.end_closed_tail(site, args);
        }
        if tag == GlobalHelper::NumericWritePrepare {
            let site = usize::try_from(site)
                .map_err(|_| invalid("numeric materialization site overflow"))?;
            return self.prepare_numeric_write(site, args);
        }
        if tag == GlobalHelper::NumericSegmentEnd {
            return self.end_numeric_segment(
                usize::try_from(site).map_err(|_| invalid("parent segment site overflow"))?,
                args,
            );
        }
        if self.closed_tail.is_some() {
            return Err(invalid(
                "ordinary helper while closed Tail fuel is outstanding",
            ));
        }
        if matches!(
            tag,
            GlobalHelper::ParallelRun
                | GlobalHelper::ParallelTake
                | GlobalHelper::ParallelYield
                | GlobalHelper::ParallelEnter
        ) {
            return self.parallel_helper(
                usize::try_from(site).map_err(|_| invalid("parallel helper site overflow"))?,
                operation,
                args,
            );
        }
        if matches!(
            tag,
            GlobalHelper::PrefixPause
                | GlobalHelper::PrefixBorrowedResume
                | GlobalHelper::PrefixTailResume
                | GlobalHelper::PrefixScanned
        ) {
            return self.prefix_helper(site, operation, args);
        }
        // Native path interiors have no callback. A selected map operation
        // admits its own observable boundary before reading any operands or
        // mutating storage. Canonical/refused execution keeps ordinary visits.
        if (tag == GlobalHelper::MapOwnerCall && self.retained_native_call(site as usize))
            || (tag == GlobalHelper::MapOwnerSpan && self.retained_native_span(site as usize))
        {
            self.visit(site)?;
        }
        if self.state.control.previous.map(u64::from) != Some(site) {
            return Err(invalid("helper outside exact admitted visit"));
        }
        let site = site as usize;
        #[cfg(all(test, feature = "canonical-native-program"))]
        crate::native_artifact::program::runtime::collections::before_span_helper(self, site)?;
        let descriptor = self.descriptor;
        if tag == GlobalHelper::ClosedTailBegin {
            return self.begin_closed_tail(site, args);
        }
        if tag == GlobalHelper::NumericSegmentBegin {
            return self.begin_numeric_segment(site, args);
        }
        if matches!(
            tag,
            GlobalHelper::MapOwnerBegin
                | GlobalHelper::MapOwnerCall
                | GlobalHelper::MapOwnerEnd
                | GlobalHelper::MapOwnerSpan
        ) {
            return self.retained_map_helper(site, operation, args);
        }
        if matches!(
            tag,
            GlobalHelper::FrozenMapCopy | GlobalHelper::FrozenMapRead
        ) {
            return self.frozen_map_helper(site, operation, args);
        }
        if self.frozen_native_action(site)
            || self.retained_native_action(site)
            || self.retained_native_call(site)
            || self.retained_native_exit(site)
            || self.retained_native_span(site)
        {
            return Err(invalid("ordinary helper at retained native map site"));
        }
        if tag == GlobalHelper::BorrowedStringsPrepare {
            return self.prepare_borrowed_strings(site, args);
        }
        if tag == GlobalHelper::BorrowedStringFailure {
            return self.borrowed_string_failure(site, args);
        }
        if self.borrowed_action(site).is_some() {
            return Err(invalid("ordinary helper at native borrowed String site"));
        }
        if descriptor
            .metadata()
            .scalar_aggregates
            .action(site)
            .is_some()
        {
            return Err(invalid("helper at native-only aggregate site"));
        }
        if tag == GlobalHelper::WindowPrepare {
            self.prepare_window_region(site, args)
        } else if tag == GlobalHelper::WindowFinish {
            self.finish_window_region(site, args)
        } else if tag == GlobalHelper::StringMapPrepare {
            self.prepare_string_map_region(site, args)
        } else if tag == GlobalHelper::StringMapFinish {
            self.finish_string_map_region(site, args)
        } else if tag == GlobalHelper::JoinMapPrepare {
            self.prepare_join_map_chunk(site, args)
        } else if tag == GlobalHelper::JoinMapFinish {
            self.finish_join_map_chunk(site, args)
        } else if self.window_region.is_some() {
            Err(invalid("helper while compiled String loans are active"))
        } else if tag == GlobalHelper::StructuralObservation {
            if self.state.consumed || args != [0; 3] {
                return Err(invalid("duplicate or malformed structural observation"));
            }
            if let Some(result) = self.buffer_frame_action(site, tag, &[])? {
                self.state.consumed = true;
                return Ok(result);
            }
            let NativeIterationAction::Structural {
                source,
                path,
                observation,
                ..
            } = descriptor.metadata().plan.blocks()[site].action()
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
            self.state.control.work.charge(
                NativeExecutionPhase::Node(
                    descriptor.metadata().plan.blocks()[site].origin().node(),
                ),
                work,
            )?;
            let root = self
                .state
                .arena
                .copy_current(source.get(), &mut self.state.control.work)?;
            if *observation == Observation::CanonicalValue {
                let NativeIterationAction::Structural { output, .. } =
                    descriptor.metadata().plan.blocks()[site].action()
                else {
                    unreachable!()
                };
                let prior = if self
                    .state
                    .arena
                    .current_retained_bytes(output.get(), &mut self.state.control.work)?
                    .is_some()
                {
                    Some(
                        self.state
                            .arena
                            .copy_current(output.get(), &mut self.state.control.work)?,
                    )
                } else {
                    None
                };
                let result = mech_engine::native_artifact::native_project_structural_value(
                    &descriptor.metadata().plan,
                    site,
                    &descriptor.metadata().schemas,
                    &root,
                    prior.as_ref(),
                    self.limits,
                    &self.state.library,
                )
                .map_err(|error| NativeProgramExecutionError::Library {
                    origin: descriptor.metadata().plan.blocks()[site].origin(),
                    error,
                })?;
                self.state
                    .arena
                    .put(output.get(), &result, &mut self.state.control.work)?;
                self.state.consumed = true;
                return descriptor.metadata().plan.values()[output.get() as usize]
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
                        .state
                        .arena
                        .copy_current(peer.get(), &mut self.state.control.work)?;
                    let ValueData::String(right) = peer.data() else {
                        return Err(invalid("structural String peer has another kind"));
                    };
                    let work = u64::try_from(left.len().max(right.len()))
                        .map_err(|_| invalid("structural String comparison work overflow"))?;
                    // Both finite cumulative compute and comparison accounts
                    // admit the complete worst-case raw-byte equality first.
                    self.state.library.charge(work, work).map_err(|error| {
                        NativeProgramExecutionError::Collection {
                            origin: descriptor.metadata().plan.blocks()[site].origin(),
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
            self.state.consumed = true;
            Ok(Some(word))
        } else if tag == GlobalHelper::Library {
            if self.state.consumed {
                return Err(invalid("duplicate library helper"));
            }
            let NativeIterationAction::LibraryCall {
                operands, output, ..
            } = descriptor.metadata().plan.blocks()[site].action()
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
            if let Some(result) = self.prepared_library_call(site, words)? {
                self.state.consumed = true;
                return Ok(result);
            }
            if let Some(result) = self.line_matrix_call(site, words)? {
                self.state.consumed = true;
                return Ok(result);
            }
            if self.captured_line_element_call(site, words)? {
                self.state.consumed = true;
                return Ok(None);
            }
            if self.selected_join_call(site, words)? {
                self.state.consumed = true;
                return Ok(self.join_map_is_fusing().then_some(1));
            }
            for (index, &operand) in operands.iter().enumerate() {
                let value = if descriptor.metadata().plan.values()[operand.get() as usize]
                    .scalar_kind()
                    .is_some()
                {
                    self.scalar(operand, words[index])?
                } else {
                    if words[index] != 0 {
                        return Err(invalid("managed operand has a native payload"));
                    }
                    self.state
                        .arena
                        .copy_current(operand.get(), &mut self.state.control.work)?
                };
                self.state.call_inputs.values_mut()[index] = Some(value);
            }
            let call = descriptor.metadata().calls[site]
                .as_ref()
                .ok_or_else(|| invalid("missing exact prepared selection"))?;
            let mut result = None;
            let outcome = self.state.call_caches.invoke(
                site,
                call,
                &self.state.call_inputs.values()[..operands.len()],
                &mut result,
                self.limits,
                &self.state.library,
            );
            // Every scalar/imported owner prepaid retirement before insertion.
            // Clear both successful and failed calls without retaining an old
            // call's owners while the next site is being constructed.
            self.state.call_inputs.values_mut()[..operands.len()].fill(None);
            outcome.map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.metadata().plan.blocks()[site].origin(),
                error,
            })?;
            let result = result.ok_or_else(|| invalid("library omitted output"))?;
            self.state
                .arena
                .put(output.get(), &result, &mut self.state.control.work)?;
            self.state.consumed = true;
            descriptor.metadata().plan.values()[output.get() as usize]
                .scalar_kind()
                .map(|kind| bits(&result, kind))
                .transpose()
        } else if tag == GlobalHelper::Move {
            if self.state.consumed {
                return Err(invalid("duplicate managed move"));
            }
            let (source, output) = self
                .needs_move(site)
                .ok_or_else(|| invalid("move outside managed copy/carrier"))?;
            let frame_move = self.buffer_frame_action(site, tag, &[])?.is_some();
            if !frame_move && self.move_joined_map(source.get(), output.get())? {
                self.state.consumed = true;
                return Ok(None);
            }
            if !frame_move {
                self.state.arena.copy_current_to(
                    source.get(),
                    output.get(),
                    &mut self.state.control.work,
                )?;
            }
            if let NativeIterationAction::Latch { iteration } =
                descriptor.metadata().plan.blocks()[site].action()
                && matches!(
                    descriptor.metadata().plan.iterations()[*iteration as usize],
                    mech_engine::native_artifact::NativeControlLoop::Tail(_)
                )
            {
                self.retire_tail_locals(*iteration, RetirementKind::TailLatch)?;
            }
            self.state.consumed = true;
            Ok(None)
        } else if tag == GlobalHelper::Collection {
            if self.state.consumed || args[2] != 0 {
                return Err(invalid("duplicate collection helper"));
            }
            let mut state = self
                .state
                .collections
                .take()
                .ok_or_else(|| invalid("collection helper without state"))?;
            let uniform_iteration = match descriptor.metadata().plan.blocks()[site].terminator() {
                NativeIterationTerminator::Header { iteration, .. } => Some(iteration),
                _ => None,
            };
            let result = if uniform_iteration.is_some() {
                state
                    .fill_uniform_words(&mut CollectionExecution::new(self), site, args[0], args[1])
                    .map(|filled| Some(u64::from(filled)))
            } else {
                state.helper(
                    &mut CollectionExecution::new(self),
                    site,
                    [args[0], args[1]],
                )
            };
            self.state.collections = Some(state);
            let result = result?;
            if let Some(iteration) = uniform_iteration
                && result == Some(1)
            {
                // The proved pure region constructed its complete result. The
                // next edge remains this same header's ordinary exit; do not
                // fabricate source visits for operations no longer executed.
                self.progress[iteration as usize * 2] = 0;
            }
            self.state.consumed = true;
            Ok(result)
        } else {
            if tag != GlobalHelper::decode(OUTPUT_BASE + self.state.output_cursor as u64)
                || (self.action_required(site) && !self.state.consumed)
                || descriptor.metadata().plan.blocks()[site].terminator()
                    != NativeIterationTerminator::Scalar(NativeMatchTerminator::Complete)
            {
                return Err(invalid("typed output outside exact completion order"));
            }
            if self.state.output_cursor >= descriptor.metadata().plan.transport_output_count() {
                return Err(invalid("extra typed output"));
            }
            let out = descriptor
                .metadata()
                .plan
                .transport_output_value(self.state.output_cursor);
            let ty = descriptor
                .metadata()
                .plan
                .transport_output_type(self.state.output_cursor);
            let value = if ty.scalar_kind().is_some() {
                self.scalar(out, args[0])?
            } else {
                self.state
                    .arena
                    .copy_current(out.get(), &mut self.state.control.work)?
            };
            let target = descriptor
                .metadata()
                .plan
                .values()
                .len()
                .checked_add(self.state.output_cursor)
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| invalid("typed output slot overflow"))?;
            self.state
                .arena
                .put(target, &value, &mut self.state.control.work)?;
            self.state.output_cursor += 1;
            Ok(None)
        }
    }
    pub(in crate::native_artifact::program::runtime) fn visit(
        &mut self,
        site: u64,
    ) -> Result<(), NativeProgramExecutionError> {
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
                .descriptor
                .metadata()
                .plan
                .activation_boundary()
                .map(u64::from)
                == Some(site)
        {
            return Err(invalid(
                "activation checkpoint must return before its fresh visit",
            ));
        }
        if let Some(previous) = self.state.control.previous
            && self.action_required(previous as usize)
            && !self.state.consumed
        {
            return Err(invalid("entry omitted current required helper"));
        }
        let previous = self.admission_predecessor(site)?;
        // A native scalar/aggregate latch has no managed move callback. Retire the
        // OTHER managed loop locals after that generated copy, before the next
        // visit, exactly as the managed latch used to do. Aggregate words own
        // no heap payload and are absent from the retirement inventory.
        if let Some(previous) = previous
            && let NativeIterationAction::Latch { iteration } =
                self.descriptor.metadata().plan.blocks()[previous as usize].action()
            && let mech_engine::native_artifact::NativeControlLoop::Tail(tail) =
                &self.descriptor.metadata().plan.iterations()[*iteration as usize]
            && (self
                .descriptor
                .metadata()
                .scalar_aggregates
                .action(previous as usize)
                .is_some()
                || self.retained_native_action(previous as usize)
                || self.descriptor.metadata().plan.values()[tail.carrier().get() as usize]
                    .scalar_kind()
                    .is_some())
        {
            self.retire_tail_locals(*iteration, RetirementKind::TailLatch)?;
        }
        let index =
            usize::try_from(site).map_err(|_| invalid("program visit site outside descriptor"))?;
        if let Some(worker) = &self.worker
            && !self
                .descriptor
                .metadata()
                .parallel
                .regions
                .get(worker.region)
                .is_some_and(|region| region.contains(index as u32))
        {
            return Err(invalid("worker visited outside its outlined source body"));
        }
        let block = self
            .descriptor
            .metadata()
            .plan
            .blocks()
            .get(index)
            .ok_or_else(|| invalid("program visit site outside descriptor"))?;
        self.validate_selected_join_boundary(index)?;
        if let Some(state) = &self.state.collections {
            state.validate_span_boundary(block.action())?;
        }
        let group = self
            .descriptor
            .metadata()
            .map_admission
            .group(index)
            .filter(|group| match group.owner {
                Some(owner) => self.state.map_owners.has_retained_binding(owner),
                None => {
                    group.start() as usize == index
                        && self.worker.is_none()
                        && self
                            .state
                            .library
                            .unscoped_remaining()
                            .is_some_and(|remaining| remaining.0 >= group.work)
                        && self
                            .state
                            .control
                            .work
                            .limit()
                            .checked_sub(self.state.control.work.used())
                            .is_some_and(|remaining| remaining >= group.work)
                }
            });
        if group.is_some_and(|group| u64::from(group.start()) != site) {
            return Err(invalid("ordinary visit inside admitted scalar group"));
        }
        if previous.is_none() && self.worker.is_some() {
            let worker = self.worker.as_ref().unwrap();
            let region = self
                .descriptor
                .metadata()
                .parallel
                .regions
                .get(worker.region)
                .ok_or_else(|| invalid("worker entry proof missing"))?;
            if site != u64::from(region.begin) || group.is_some() {
                return Err(invalid("worker entry outside constructor proof"));
            }
            self.control().visit_outlined_entry(site, region.begin)?;
        } else {
            self.control()
                .visit_from(site, group.map(|group| group.work), previous)?;
        }
        self.state.scalar_path = group
            .filter(|group| group.owner.is_none())
            .map(|group| group.start());
        self.state.map_owners.closed_exit = None;
        let descriptor = self.descriptor;
        if let Some(mech_engine::native_artifact::NativeBorrowedStringAction::Equal { .. }) =
            self.borrowed_action(index)
        {
            self.state.library.charge(32, 32).map_err(|error| {
                NativeProgramExecutionError::Collection {
                    origin: block.origin(),
                    error,
                }
            })?;
        }
        if let crate::native_artifact::iteration_abi::Progress::Exit(iteration) =
            descriptor.metadata().visits[index].progress
            && self.borrowed_strings.as_ref().is_some_and(|state| {
                descriptor.metadata().borrowed_strings.regions[state.region]
                    .proof
                    .iteration()
                    == iteration
            })
        {
            self.borrowed_strings = None;
        }
        if let crate::native_artifact::iteration_abi::Progress::Exit(iteration) =
            descriptor.metadata().visits[site as usize].progress
            && matches!(
                descriptor.metadata().plan.iterations()[iteration as usize],
                mech_engine::native_artifact::NativeControlLoop::Tail(_)
            )
        {
            // Keep only the final return until the following generated copy.
            self.retire_tail_locals(iteration, RetirementKind::TailExit)?;
        }
        if let crate::native_artifact::iteration_abi::Progress::Exit(iteration) =
            descriptor.metadata().visits[site as usize].progress
            && matches!(
                descriptor.metadata().plan.iterations()[iteration as usize],
                mech_engine::native_artifact::NativeControlLoop::Collection(_)
            )
        {
            let mut state = self
                .state
                .collections
                .take()
                .ok_or_else(|| invalid("collection exit without state"))?;
            let result = state.release_source(&mut CollectionExecution::new(self), iteration);
            self.state.collections = Some(state);
            result?;
        }
        if self.state.control.completed
            && self
                .state
                .collections
                .as_ref()
                .is_some_and(|state| !state.is_complete())
        {
            return Err(invalid("completion with an active collection builder"));
        }
        if self.state.control.completed && self.has_retained_maps() {
            return Err(invalid("completion with an active retained map owner"));
        }
        self.state.consumed = false;
        // Keep the actual admitted anchor. The next callback must belong to
        // this prepaid path's exact frontier; interior failures retain their
        // original source sites. No synthetic per-instruction visits occur.
        Ok(())
    }

    /// Read the same proved predecessor at visits and phase checkpoints.
    /// A paid entry remains stored until the next boundary is actually visited.
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn admission_predecessor(
        &self,
        site: u64,
    ) -> Result<Option<u32>, NativeProgramExecutionError> {
        Ok(
            if let Some((from, to)) = self.state.map_owners.closed_exit {
                if self.state.control.previous != Some(from) || site != u64::from(to) {
                    return Err(invalid("visit outside closed map exit frontier"));
                }
                Some(from)
            } else if let Some(previous) = self.state.control.previous
                && let Some(group) = self
                    .descriptor
                    .metadata()
                    .map_admission
                    .group(previous as usize)
                && self.admission_active(group)
            {
                if previous != group.start() {
                    return Err(invalid("native path lost its entry"));
                }
                Some(
                    group
                        .predecessor(
                            u32::try_from(site)
                                .map_err(|_| invalid("map path frontier overflow"))?,
                        )
                        .ok_or_else(|| invalid("visit outside prepaid native path frontier"))?,
                )
            } else {
                self.state.control.previous
            },
        )
    }

    fn admission_active(
        &self,
        group: &crate::native_artifact::program::prepare::map_admission::Group,
    ) -> bool {
        match group.owner {
            Some(owner) => self.state.map_owners.has_retained_binding(owner),
            None => self.state.scalar_path == Some(group.start()),
        }
    }

    /// The existing one-word callback result is scratch between helpers.
    /// Scalar interiors contain no helper that could overwrite this decision.
    pub(in crate::native_artifact::program::runtime) fn scalar_admission_word(
        &self,
        site: u64,
    ) -> Option<u64> {
        self.descriptor
            .metadata()
            .map_admission
            .group(site as usize)
            .filter(|group| group.owner.is_none())
            .map(|group| u64::from(self.state.scalar_path == Some(group.start())))
    }

    pub(in crate::native_artifact::program::runtime) fn failure_site(
        &self,
        raw: u64,
    ) -> Result<u32, NativeProgramExecutionError> {
        let previous = self
            .state
            .control
            .previous
            .ok_or_else(|| invalid("program failure before visit"))?;
        let site = u32::try_from(raw).map_err(|_| invalid("program failure origin mismatch"))?;
        if site == previous {
            return Ok(site);
        }
        let layout = &self.descriptor.metadata().map_admission;
        if let Some(group) = layout.group(previous as usize)
            && self.admission_active(group)
            && group.start() == previous
            && group.sites.contains(&site)
        {
            return Ok(site);
        }
        Err(invalid("program failure origin mismatch"))
    }

    pub(in crate::native_artifact::program::runtime) fn retire_tail_locals(
        &mut self,
        iteration: u32,
        kind: RetirementKind,
    ) -> Result<(), NativeProgramExecutionError> {
        self.retire_keys(iteration)?;
        self.retire_frozen(iteration)?;
        let slots = self
            .descriptor
            .metadata()
            .retirements
            .slots(self.retirements, iteration, kind)
            .map_err(invalid)?;
        let scan = amount(slots.len(), 16, 64)?;
        self.state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, scan)?;
        for &slot in slots {
            self.state.buffer_frames.retire(
                &self.descriptor.metadata().frame_retirements,
                slot as u32,
                &mut self.state.control.work,
            )?;
            self.state
                .arena
                .retire_current(slot as u32, &mut self.state.control.work)?;
        }
        Ok(())
    }
}

/// The collection owner receives only admitted values, its input bank, ledger
/// and the exact continuation operations needed by existing selected sinks.
pub(in crate::native_artifact::program::runtime) struct CollectionExecution<
    'borrow,
    'state,
    'visit,
    'plan,
> {
    invocation: &'borrow mut Invocation<'state, 'visit, 'plan>,
}
impl<'borrow, 'state, 'visit, 'plan> CollectionExecution<'borrow, 'state, 'visit, 'plan> {
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn new(
        invocation: &'borrow mut Invocation<'state, 'visit, 'plan>,
    ) -> Self {
        Self { invocation }
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn descriptor(
        &self,
    ) -> &'plan NativeProgramDescriptor {
        self.invocation.descriptor
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn work(&mut self) -> &mut NativeWorkMeter {
        &mut self.invocation.state.control.work
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn library(&self) -> &NativeLibraryWorkBudget {
        &self.invocation.state.library
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn limits(&self) -> MemoryBudgetLimits {
        self.invocation.limits
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn caches(&self) -> &CallCacheLane<'plan> {
        &self.invocation.state.call_caches
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn caches_and_library(
        &mut self,
    ) -> (&mut CallCacheLane<'plan>, &NativeLibraryWorkBudget) {
        (
            &mut self.invocation.state.call_caches,
            &self.invocation.state.library,
        )
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_copy_current(
        &mut self,
        slot: u32,
    ) -> Result<Value, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.copy_current(slot, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_current_handle(
        &mut self,
        slot: u32,
    ) -> Result<NativeValueHandle, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.current_handle(slot, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_get(
        &mut self,
        handle: NativeValueHandle,
    ) -> Result<&Value, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.get(handle, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_put(
        &mut self,
        slot: u32,
        value: &Value,
    ) -> Result<NativeValueHandle, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.put(slot, value, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn scalar(
        &mut self,
        value: NativeMatchValue,
        word: u64,
    ) -> Result<Value, Error> {
        self.invocation.scalar(value, word)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn ledger(&self) -> &ControlState {
        &self.invocation.state.control
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn control(
        &mut self,
    ) -> Control<'_, '_, NativeProgramDescriptor> {
        self.invocation.control()
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn inputs(&self) -> &[Option<Value>] {
        self.invocation.state.call_inputs.values()
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn inputs_mut(
        &mut self,
    ) -> &mut [Option<Value>] {
        self.invocation.state.call_inputs.values_mut()
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn progress(&self) -> &[u64] {
        self.invocation.progress
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn retirements(&self) -> &'visit [u64] {
        self.invocation.retirements
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn worker(&self) -> &Option<parallel::Worker> {
        &self.invocation.worker
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn retire_frozen(
        &mut self,
        iteration: u32,
    ) -> Result<(), Error> {
        self.invocation.retire_frozen(iteration)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn initialize_joined_map(
        &mut self,
        iteration: u32,
        source: u32,
    ) -> Result<Option<u64>, Error> {
        self.invocation.initialize_joined_map(iteration, source)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn take_joined_map(
        &mut self,
        region: &mech_engine::native_artifact::NativeStringMapRegion,
    ) -> Result<Option<joins::Mapped>, Error> {
        self.invocation.take_joined_map(region)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_current_retained_bytes(
        &mut self,
        slot: u32,
    ) -> Result<Option<u64>, Error> {
        let state = &mut *self.invocation.state;
        Ok(state
            .arena
            .current_retained_bytes(slot, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_retire_current(
        &mut self,
        slot: u32,
    ) -> Result<(), Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.retire_current(slot, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_retire_current_matching(
        &mut self,
        slot: u32,
        owner: &Value,
    ) -> Result<(), Error> {
        let state = &mut *self.invocation.state;
        Ok(state
            .arena
            .retire_current_matching(slot, owner, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn arena_take(
        &mut self,
        handle: NativeValueHandle,
    ) -> Result<Value, Error> {
        let state = &mut *self.invocation.state;
        Ok(state.arena.take(handle, &mut state.control.work)?)
    }
    #[inline]
    pub(in crate::native_artifact::program::runtime) fn charge_input_bank_headers(
        &mut self,
    ) -> Result<(), Error> {
        let state = &mut *self.invocation.state;
        collections::charge(
            &mut state.control.work,
            amount(state.call_inputs.values().len(), 16, 128)?,
        )
    }
}
/// Read-only accounting/observation access is statically dispatched. It cannot
/// lend mutable phase state or manufacture a native visit or resource permit.
pub(in crate::native_artifact::program::runtime) trait CollectionRead {
    fn descriptor(&self) -> &NativeProgramDescriptor;
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn library(&self) -> &NativeLibraryWorkBudget;
    fn limits(&self) -> MemoryBudgetLimits;
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn ledger(&self) -> &ControlState;
    fn progress(&self) -> &[u64];
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn arena_test_current_value(&self, slot: u32) -> Option<&Value>;
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn arena_test_handle_value(&self, handle: NativeValueHandle) -> Option<&Value>;
}
impl CollectionRead for Invocation<'_, '_, '_> {
    #[inline]
    fn progress(&self) -> &[u64] {
        self.progress
    }

    #[inline]
    fn descriptor(&self) -> &NativeProgramDescriptor {
        self.descriptor
    }
    #[inline]
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn library(&self) -> &NativeLibraryWorkBudget {
        &self.state.library
    }
    #[inline]
    fn limits(&self) -> MemoryBudgetLimits {
        self.limits
    }
    #[inline]
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn ledger(&self) -> &ControlState {
        &self.state.control
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn arena_test_current_value(&self, slot: u32) -> Option<&Value> {
        self.state.arena.test_current_value(slot)
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn arena_test_handle_value(&self, handle: NativeValueHandle) -> Option<&Value> {
        self.state.arena.test_handle_value(handle)
    }
}
impl CollectionRead for CollectionExecution<'_, '_, '_, '_> {
    #[inline]
    fn progress(&self) -> &[u64] {
        self.invocation.progress
    }

    #[inline]
    fn descriptor(&self) -> &NativeProgramDescriptor {
        self.invocation.descriptor
    }
    #[inline]
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn library(&self) -> &NativeLibraryWorkBudget {
        &self.invocation.state.library
    }
    #[inline]
    fn limits(&self) -> MemoryBudgetLimits {
        self.invocation.limits
    }
    #[inline]
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn ledger(&self) -> &ControlState {
        &self.invocation.state.control
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn arena_test_current_value(&self, slot: u32) -> Option<&Value> {
        self.invocation.state.arena.test_current_value(slot)
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    fn arena_test_handle_value(&self, handle: NativeValueHandle) -> Option<&Value> {
        self.invocation.state.arena.test_handle_value(handle)
    }
}

pub(in crate::native_artifact::program::runtime) mod borrowed;

pub(in crate::native_artifact::program::runtime) mod caller_geometry;

pub(in crate::native_artifact::program::runtime) mod tail;

pub(in crate::native_artifact::program::runtime) mod parallel;

pub(in crate::native_artifact::program::runtime) mod joins;

pub(in crate::native_artifact::program::runtime) mod prefix;

pub(in crate::native_artifact::program::runtime) mod window;
