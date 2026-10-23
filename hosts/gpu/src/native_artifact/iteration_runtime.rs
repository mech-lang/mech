//! One admitted managed invocation. Callback checks visits and loop progress;
//! every scalar condition, arithmetic operation and carrier transition executes
//! in generated machine code. There is no resident/program evaluator here.
use super::abi::{
    NO_NODE, NativeEntryFrame as Frame, STATUS_ABI, STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK,
};
use super::iteration_abi::{NativeIterationDescriptor, Progress, STATUS_REJECT, VERSION};
use super::memory::{NativeWordStorage, transport_bytes};
use super::{NativeExecutionError, NativeExecutionLimits, NativeExecutionPhase, NativeWorkMeter};
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{NativeMatchOrigin, NativeScalarKind, NativeScalarValue};
use std::ffi::c_void;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeIterationExecutionError {
    Base(NativeExecutionError),
    Work {
        origin: Option<NativeMatchOrigin>,
        required: u64,
        limit: u64,
    },
    Visit {
        previous: Option<u32>,
        actual: u64,
    },
    Arithmetic {
        origin: NativeMatchOrigin,
    },
    Unmatched {
        origin: NativeMatchOrigin,
    },
}
impl From<NativeExecutionError> for NativeIterationExecutionError {
    fn from(value: NativeExecutionError) -> Self {
        Self::Base(value)
    }
}
impl std::fmt::Display for NativeIterationExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native iteration execution: {self:?}")
    }
}
impl std::error::Error for NativeIterationExecutionError {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeIterationExecutionReport {
    pub work: u64,
    pub transport_bytes: u64,
    pub visits: u64,
}
fn invalid(reason: &'static str) -> NativeIterationExecutionError {
    NativeExecutionError::Descriptor { reason }.into()
}
fn add(a: u64, b: u64) -> Result<u64, NativeIterationExecutionError> {
    a.checked_add(b)
        .ok_or_else(|| invalid("iteration count/work overflow"))
}
fn mul(a: u64, b: u64) -> Result<u64, NativeIterationExecutionError> {
    a.checked_mul(b)
        .ok_or_else(|| invalid("iteration count/work overflow"))
}
fn charge(
    meter: &mut NativeWorkMeter,
    amount: u64,
    origin: Option<NativeMatchOrigin>,
) -> Result<(), NativeIterationExecutionError> {
    meter
        .charge(
            origin.map_or(NativeExecutionPhase::Setup, |origin| {
                NativeExecutionPhase::Node(origin.node())
            }),
            amount,
        )
        .map_err(|error| match error {
            NativeExecutionError::WorkLimit {
                required, limit, ..
            } => NativeIterationExecutionError::Work {
                origin,
                required,
                limit,
            },
            error => error.into(),
        })
}
pub(super) trait VisitPlan {
    fn entry(&self) -> u32;
    fn visits(&self) -> &[super::iteration_abi::NativeIterationVisit];
    #[cfg(feature = "canonical-native-program-runtime")]
    fn iteration_count(&self) -> usize;
    fn iteration(&self, index: usize) -> mech_engine::native_artifact::NativeControlLoopRef<'_>;
}
impl VisitPlan for NativeIterationDescriptor {
    fn entry(&self) -> u32 {
        self.entry
    }
    fn visits(&self) -> &[super::iteration_abi::NativeIterationVisit] {
        &self.visits
    }
    #[cfg(feature = "canonical-native-program-runtime")]
    fn iteration_count(&self) -> usize {
        self.iterations.len()
    }
    fn iteration(&self, index: usize) -> mech_engine::native_artifact::NativeControlLoopRef<'_> {
        (&self.iterations[index]).into()
    }
}
pub(super) struct Context<'a, D: VisitPlan> {
    pub(super) descriptor: &'a D,
    // Separate managed backing from generated code's locals/counters; no raw
    // machine pointer aliases this exclusive callback progress slice.
    pub(super) progress: &'a mut [u64],
    pub(super) current: Option<u32>,
    pub(super) previous: Option<u32>,
    pub(super) completed: bool,
    pub(super) work: NativeWorkMeter,
    pub(super) visits: u64,
    pub(super) error: Option<NativeIterationExecutionError>,
}
impl<D: VisitPlan> Context<'_, D> {
    pub(super) fn visit(&mut self, raw: u64) -> Result<(), NativeIterationExecutionError> {
        self.visit_with_work(raw, None)
    }

    // A build-proved straight-line native group can fund its whole body at
    // the original entry. The ordinary scalar/iteration ABI uses None.
    pub(super) fn visit_with_work(&mut self, raw: u64, group_work: Option<u64>)
        -> Result<(), NativeIterationExecutionError>
    {
        self.visit_from(raw, group_work, self.previous)
    }

    // A paid native path supplies its proved frontier predecessor. Keep the
    // stored anchor unchanged until the next boundary has been fully admitted.
    pub(super) fn visit_from(&mut self, raw: u64, group_work: Option<u64>, previous: Option<u32>)
        -> Result<(), NativeIterationExecutionError>
    {
        let id = u32::try_from(raw).ok();
        let allowed = !self.completed
            && match previous {
                None => id == Some(self.descriptor.entry()),
                Some(previous) => {
                    let prior = &self.descriptor.visits()[previous as usize];
                    if let Progress::Header(iteration) = prior.progress {
                        if matches!(
                            self.descriptor.iteration(iteration as usize),
                            mech_engine::native_artifact::NativeControlLoopRef::Tail(_)
                        ) {
                            id == prior.successors[0]
                        } else {
                            let remaining = self.progress[iteration as usize * 2];
                            id == prior.successors[usize::from(remaining == 0)]
                        }
                    } else {
                        id.is_some() && prior.successors.contains(&id)
                    }
                }
            };
        if !allowed {
            return Err(NativeIterationExecutionError::Visit {
                previous: self.previous,
                actual: raw,
            });
        }
        let id = id.ok_or_else(|| invalid("iteration visit id overflow"))?;
        let site = *self
            .descriptor
            .visits()
            .get(id as usize)
            .ok_or_else(|| invalid("iteration visit outside descriptor"))?;
        // Charge before reading/changing per-loop metadata or inspecting any
        // completion progress words. The generated block is not entered yet.
        let work = group_work.unwrap_or(site.work);
        if work < site.work { return Err(invalid("group admission underfunds its entry")); }
        charge(&mut self.work, work, Some(site.origin))?;
        match site.progress {
            Progress::Initialize(iteration) => {
                let data = self.descriptor.iteration(iteration as usize);
                if self.current != data.parent() || self.progress[iteration as usize * 2 + 1] != 0 {
                    return Err(invalid(
                        "iteration initialized outside exact parent invocation",
                    ));
                }
                match data {
                    mech_engine::native_artifact::NativeControlLoopRef::Iteration(data) => {
                        self.progress[iteration as usize * 2] = data.count();
                        self.progress[iteration as usize * 2 + 1] = 1;
                    }
                    mech_engine::native_artifact::NativeControlLoopRef::Collection(_) => {
                        // Only the exact admitted initializer helper may bind
                        // a live collection's count before entering its header.
                        self.progress[iteration as usize * 2] = 0;
                        self.progress[iteration as usize * 2 + 1] = 2;
                    }
                    mech_engine::native_artifact::NativeControlLoopRef::Tail(_) => {
                        self.progress[iteration as usize * 2] = 0;
                        self.progress[iteration as usize * 2 + 1] = 1;
                    }
                }
                self.current = Some(iteration);
            }
            Progress::Exit(iteration) => {
                if self.current != Some(iteration)
                    || self.progress[iteration as usize * 2] != 0
                    || self.progress[iteration as usize * 2 + 1] != 1
                {
                    return Err(invalid("iteration exited before exact count"));
                }
                self.progress[iteration as usize * 2 + 1] = 0;
                self.current = self.descriptor.iteration(iteration as usize).parent();
                if self.current != site.iteration {
                    return Err(invalid("iteration exit parent mismatch"));
                }
            }
            Progress::Latch(iteration) => {
                let tail = matches!(
                    self.descriptor.iteration(iteration as usize),
                    mech_engine::native_artifact::NativeControlLoopRef::Tail(_)
                );
                if self.current != Some(iteration)
                    || self.progress[iteration as usize * 2 + 1] != 1
                    || (!tail && self.progress[iteration as usize * 2] == 0)
                {
                    return Err(invalid("iteration latch outside active body"));
                }
                if !tail {
                    self.progress[iteration as usize * 2] -= 1;
                }
            }
            Progress::Header(iteration) => {
                if self.current != Some(iteration) || self.progress[iteration as usize * 2 + 1] != 1
                {
                    return Err(invalid("loop header before admitted live count"));
                }
            }
            Progress::Normal => {
                if self.current != site.iteration {
                    return Err(invalid("iteration visit outside lexical invocation"));
                }
            }
        }
        if site.complete
            && (self.current.is_some() || self.progress.chunks_exact(2).any(|v| v[1] != 0))
        {
            return Err(invalid("completion with active iteration"));
        }
        self.previous = Some(id);
        self.completed = site.complete;
        self.visits = add(self.visits, 1)?;
        Ok(())
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    pub(super) fn bind_collection_count(
        &mut self,
        iteration: u32,
        count: u64,
    ) -> Result<(), NativeIterationExecutionError> {
        if iteration as usize >= self.descriptor.iteration_count()
            || !matches!(
                self.descriptor.iteration(iteration as usize),
                mech_engine::native_artifact::NativeControlLoopRef::Collection(_)
            )
            || self.current != Some(iteration)
            || self
                .previous
                .map(|site| self.descriptor.visits()[site as usize].progress)
                != Some(Progress::Initialize(iteration))
            || self.progress[iteration as usize * 2 + 1] != 2
        {
            return Err(invalid("live count outside exact collection initializer"));
        }
        charge(&mut self.work, 32, None)?;
        self.progress[iteration as usize * 2] = count;
        self.progress[iteration as usize * 2 + 1] = 1;
        Ok(())
    }
}
unsafe extern "C" fn admit(context: *mut c_void, id: u64) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: execute owns context and disjoint progress backing synchronously.
    let context = unsafe { &mut *context.cast::<Context<'_, NativeIterationDescriptor>>() };
    if context.error.is_some() {
        return STATUS_ADMISSION;
    }
    match context.visit(id) {
        Ok(()) => STATUS_OK,
        Err(error) => {
            context.error = Some(error);
            STATUS_ADMISSION
        }
    }
}
pub(super) fn execute<F: FnOnce(*mut Frame) -> u64>(
    descriptor: &NativeIterationDescriptor,
    entry: F,
    inputs: &[NativeScalarValue],
    outputs: &mut [NativeScalarValue],
    limits: NativeExecutionLimits,
    budget: Option<&ManagedMemoryBudget>,
) -> Result<NativeIterationExecutionReport, NativeIterationExecutionError> {
    if inputs.len() != descriptor.inputs.len() {
        return Err(NativeExecutionError::InvalidInputs {
            expected: descriptor.inputs.len(),
            actual: inputs.len(),
        }
        .into());
    }
    if outputs.len() != descriptor.outputs.len() {
        return Err(NativeExecutionError::InvalidOutputs {
            expected: descriptor.outputs.len(),
            actual: outputs.len(),
        }
        .into());
    }
    if descriptor.stack_bytes > limits.max_native_stack_bytes {
        return Err(NativeExecutionError::StackLimit {
            required: descriptor.stack_bytes,
            limit: limits.max_native_stack_bytes,
        }
        .into());
    }
    let locals = descriptor
        .local_count()
        .ok_or_else(|| invalid("iteration local count overflow"))?;
    let progress = descriptor
        .iterations
        .len()
        .checked_mul(2)
        .ok_or_else(|| invalid("iteration progress count overflow"))?;
    let words = inputs
        .len()
        .checked_add(locals)
        .and_then(|v| v.checked_add(progress))
        .and_then(|v| v.checked_add(outputs.len()))
        .ok_or_else(|| invalid("iteration transport overflow"))?;
    let bytes = transport_bytes(words)?;
    if bytes > limits.max_transport_bytes {
        return Err(NativeExecutionError::TransportLimit {
            required: bytes,
            limit: limits.max_transport_bytes,
        }
        .into());
    }
    let setup = add(
        256,
        add(
            mul(words as u64, 4)?,
            add(
                mul(inputs.len() as u64, 16)?,
                mul(descriptor.constant_count as u64, 8)?,
            )?,
        )?,
    )?;
    let mut work = NativeWorkMeter::new(limits.max_work)?;
    charge(&mut work, setup, None)?;
    // Actual shared memory denial precedes input type scans as well as copies.
    let mut storage = NativeWordStorage::new(words, limits.max_transport_bytes, budget)?;
    for (index, (value, ty)) in inputs.iter().zip(&descriptor.inputs).enumerate() {
        if !matches!(
            (value, ty.kind()),
            (NativeScalarValue::Bool(_), NativeScalarKind::Bool)
                | (NativeScalarValue::U64(_), NativeScalarKind::U64)
                | (NativeScalarValue::I64(_), NativeScalarKind::I64)
                | (NativeScalarValue::F64(_), NativeScalarKind::F64)
        ) {
            return Err(NativeExecutionError::InputType {
                input: index,
                expected: ty.kind(),
            }
            .into());
        }
    }
    let (input_words, rest) = storage.values_mut().split_at_mut(inputs.len());
    let (values, rest) = rest.split_at_mut(locals);
    let (progress_words, output_words) = rest.split_at_mut(progress);
    for (word, value) in input_words.iter_mut().zip(inputs) {
        *word = match value {
            NativeScalarValue::Bool(v) => u64::from(*v),
            NativeScalarValue::U64(v) => *v,
            NativeScalarValue::I64(v) => *v as u64,
            NativeScalarValue::F64(v) => v.bits(),
        };
    }
    let input_pointer = input_words.as_ptr();
    let local_pointer = values.as_mut_ptr();
    let output_pointer = output_words.as_mut_ptr();
    let mut context = Context {
        descriptor,
        progress: progress_words,
        current: None,
        previous: None,
        completed: false,
        work,
        visits: 0,
        error: None,
    };
    let context_pointer =
        (&mut context as *mut Context<'_, NativeIterationDescriptor>).cast::<c_void>();
    let mut frame = Frame {
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
    };
    let status = entry(&mut frame);
    if let Some(error) = context.error.take() {
        return Err(error);
    }
    if frame.version != VERSION
        || frame.input_count != inputs.len() as u64
        || frame.local_count != locals as u64
        || frame.output_count != outputs.len() as u64
        || frame.inputs != input_pointer
        || frame.locals != local_pointer
        || frame.outputs != output_pointer
        || frame.context != context_pointer
        || frame.admit_next as usize != admit as *const () as usize
    {
        return Err(invalid("iteration entry changed immutable frame"));
    }
    match status {
        STATUS_OK => {
            if !context.completed || frame.error_node != NO_NODE {
                return Err(invalid("iteration omitted atomic completion"));
            }
        }
        STATUS_ARITHMETIC | STATUS_REJECT => {
            let id = context
                .previous
                .ok_or_else(|| invalid("iteration failure before admitted visit"))?;
            let site = &descriptor.visits[id as usize];
            if frame.error_node != id as u64 {
                return Err(invalid("iteration failure origin mismatch"));
            }
            if status == STATUS_ARITHMETIC {
                if !site.arithmetic {
                    return Err(invalid("iteration arithmetic outside primitive"));
                }
                return Err(NativeIterationExecutionError::Arithmetic {
                    origin: site.origin,
                });
            }
            if !site.reject {
                return Err(invalid("iteration unmatched status outside rejection"));
            }
            return Err(NativeIterationExecutionError::Unmatched {
                origin: site.origin,
            });
        }
        _ => return Err(NativeExecutionError::InvalidStatus { status }.into()),
    }
    for (index, (&word, ty)) in output_words.iter().zip(&descriptor.outputs).enumerate() {
        if ty.kind() == NativeScalarKind::Bool && word > 1 {
            return Err(NativeExecutionError::InvalidOutputBool {
                output: index,
                value: word,
            }
            .into());
        }
    }
    // All fallible checks precede this allocation-free final publication pass.
    for ((target, &word), ty) in outputs
        .iter_mut()
        .zip(output_words.iter())
        .zip(&descriptor.outputs)
    {
        *target = match ty.kind() {
            NativeScalarKind::Bool => NativeScalarValue::Bool(word != 0),
            NativeScalarKind::U64 => NativeScalarValue::U64(word),
            NativeScalarKind::I64 => NativeScalarValue::I64(word as i64),
            NativeScalarKind::F64 => {
                NativeScalarValue::F64(mech_core::snapshot::F64Bits::from_bits(word))
            }
        };
    }
    Ok(NativeIterationExecutionReport {
        work: context.work.used(),
        transport_bytes: bytes,
        visits: context.visits,
    })
}

#[cfg(all(test, feature = "canonical-native"))]
mod tests {
    use super::*;
    use mech_engine::CanonicalSourceFrontend;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    fn plan(count: u64) -> mech_engine::native_artifact::NativeIterationArtifactPlan {
        let source = format!(
            "identity(x<u64>) => (next<u64>) := next := x.\nseed<u64> |{count}> identity\n"
        );
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x49545254), Revision(1), source.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let catalog = mech_stdlib::source_catalog();
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        mech_engine::native_artifact::lower_scalar_iteration_artifact(&artifact, &catalog).unwrap()
    }
    #[test]
    fn callback_progress_rejects_early_exit_and_body_after_exact_count() {
        for count in [0, 2] {
            let plan = plan(count);
            let descriptor = NativeIterationDescriptor::from_plan(&plan, 32, 1).unwrap();
            let iteration = descriptor.iterations[0].clone();
            let mut progress = [0; 2];
            let mut context = Context {
                descriptor: &descriptor,
                progress: &mut progress,
                current: None,
                previous: None,
                completed: false,
                work: NativeWorkMeter::new(100_000).unwrap(),
                visits: 0,
                error: None,
            };
            context.visit(descriptor.entry as u64).unwrap();
            context.visit(iteration.initialize() as u64).unwrap();
            context.visit(iteration.header() as u64).unwrap();
            let denied = if count == 0 {
                iteration.body()
            } else {
                iteration.exit()
            };
            let before = (context.work.used(), context.visits, context.progress[0]);
            assert!(matches!(
                context.visit(denied as u64),
                Err(NativeIterationExecutionError::Visit { .. })
            ));
            assert_eq!(
                (context.work.used(), context.visits, context.progress[0]),
                before
            );
            // Identity's body has one no-op block. No hidden execution or
            // inferred effect is used to move this private progress counter.
            for _ in 0..count {
                context.visit(iteration.body() as u64).unwrap();
                context.visit(iteration.latch() as u64).unwrap();
                context.visit(iteration.header() as u64).unwrap();
            }
            context.visit(iteration.exit() as u64).unwrap();
            assert_eq!(context.current, None);
            assert_eq!(&*context.progress, &[0, 0]);
            assert!(context.visit(iteration.header() as u64).is_err());
        }
    }
    #[test]
    fn setup_transport_and_shared_budget_deny_before_input_scan_or_entry() {
        let plan = plan(2);
        let descriptor = NativeIterationDescriptor::from_plan(&plan, 32, 1).unwrap();
        for (limits, shared, expected) in [
            (
                NativeExecutionLimits {
                    max_work: 0,
                    ..NativeExecutionLimits::default()
                },
                None,
                "work",
            ),
            (
                NativeExecutionLimits {
                    max_transport_bytes: 0,
                    ..NativeExecutionLimits::default()
                },
                None,
                "transport",
            ),
            (
                NativeExecutionLimits::default(),
                Some(ManagedMemoryBudget::new(0)),
                "memory",
            ),
        ] {
            let called = std::cell::Cell::new(false);
            let mut output = [NativeScalarValue::U64(99)];
            // Wrong input type would fail the payload-type pass; denial must
            // win first without invoking the injected entry.
            let error = execute(
                &descriptor,
                |_| {
                    called.set(true);
                    STATUS_ABI
                },
                &[NativeScalarValue::Bool(false)],
                &mut output,
                limits,
                shared.as_ref(),
            )
            .unwrap_err();
            assert!(!called.get());
            assert_eq!(output, [NativeScalarValue::U64(99)]);
            match expected {
                "work" => assert!(matches!(
                    error,
                    NativeIterationExecutionError::Work { origin: None, .. }
                )),
                "transport" => assert!(matches!(
                    error,
                    NativeIterationExecutionError::Base(
                        NativeExecutionError::TransportLimit { .. }
                    )
                )),
                "memory" => assert!(matches!(
                    error,
                    NativeIterationExecutionError::Base(NativeExecutionError::Memory { .. })
                )),
                _ => unreachable!(),
            }
            if let Some(shared) = shared {
                assert_eq!(shared.used_bytes(), 0);
            }
        }
    }
}
