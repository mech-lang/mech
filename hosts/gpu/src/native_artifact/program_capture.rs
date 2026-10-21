//! Own the same selected line producer across generated scan continuations.
//! Capture storage is private; no unfinished String, Matrix, or count enters
//! the canonical arena. Runtime delivery/EOF authorization is wired separately.
use super::*;
#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
use mech_engine::native_artifact::{NativeTextLineChunks, NativeTextLineMatrix};
#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
use std::rc::Rc;

impl Context<'_, '_> {
    pub(in crate::native_artifact) fn captured_line_element_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<bool, NativeProgramExecutionError> {
        use mech_engine::native_artifact::NativeBorrowedStringAction;
        let descriptor = self.control.descriptor;
        let Some((_, NativeBorrowedStringAction::MatrixElement { source, output, .. })) =
            descriptor.borrowed_strings.action(site)
        else {
            return Ok(false);
        };
        let Some(window) = self.prefix_line_window(*source) else {
            return Ok(false);
        };
        let [0, index] = words else {
            return Err(invalid(
                "captured line element has invalid managed operands",
            ));
        };
        let call = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("captured line element lost its selected factory"))?;
        use mech_core::snapshot::{
            SnapshotCanonicalizationBudget, prospective_string_value_footprint,
        };
        use mech_core::{
            MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase,
        };
        let failure = |error| NativeProgramExecutionError::Library {
            origin: descriptor.plan.blocks()[site].origin(),
            error,
        };
        let compute_limit = self
            .limits
            .max_compute_work
            .ok_or_else(|| invalid("captured cell requires a finite compute allowance"))?;
        let comparison_limit = self
            .limits
            .max_comparison_work
            .ok_or_else(|| invalid("captured cell requires a finite comparison allowance"))?;
        // Cloning preserves the SAME invocation account. Its outer call scope
        // includes selection, Unicode materialization and arena publication;
        // the nested private projection cannot refill this leaf's allowance.
        let library = self.library.clone();
        library
            .with_call_limits(compute_limit, comparison_limit, || {
                let prior = self.selected_join_coexistence(Some(*source))?;
                let text = call
                    .text_line_window_element(window, *index, self.limits, &library, prior)
                    .map_err(failure)?;
                let payload = text.len() as u64;
                let footprint = prospective_string_value_footprint(0, false, 1, payload)
                    .map_err(|_| invalid("captured cell output footprint overflow"))?;
                // Use the same canonical String construction envelope as the
                // selected join: real output, copy/finalization staging and import
                // coexist with the still-pinned packed source and row loan.
                let envelope = amount(text.len(), 4, descriptor.scalar_bytes)?
                    .checked_add(4096)
                    .ok_or_else(|| invalid("captured cell staging overflow"))?;
                let finalization = amount(text.len(), 8, 32)?;
                let compute = envelope
                    .checked_mul(16)
                    .and_then(|work| work.checked_add(finalization))
                    .and_then(|work| work.checked_add(1024))
                    .ok_or_else(|| invalid("captured cell compute overflow"))?;
                let temporary = prior
                    .turn_peak_bytes
                    .checked_add(window.retained_capacity_bytes())
                    .and_then(|bytes| bytes.checked_add(envelope))
                    .ok_or_else(|| invalid("captured cell temporary footprint overflow"))?;
                let nodes = prior
                    .retained_nodes
                    .checked_add(9)
                    .ok_or_else(|| invalid("captured cell retained nodes overflow"))?;
                for (dimension, required, limit) in [
                    (
                        Dimension::OutputElements,
                        1,
                        self.limits.max_output_elements,
                    ),
                    (
                        Dimension::OutputBytes,
                        footprint.retained_bytes,
                        self.limits.max_output_bytes,
                    ),
                    (
                        Dimension::ClonedBytes,
                        amount(text.len(), 2, 0)?,
                        self.limits.max_cloned_bytes,
                    ),
                    (
                        Dimension::TemporaryBytes,
                        temporary,
                        self.limits.max_temporary_bytes,
                    ),
                    (
                        Dimension::RetainedNodes,
                        nodes,
                        self.limits.max_retained_nodes,
                    ),
                ] {
                    let limit =
                        limit.ok_or_else(|| invalid("captured cell requires finite limits"))?;
                    if required > limit {
                        return Err(failure(
                            ResidentKernelError::ResourceLimit {
                                phase: ResidentResourcePhase::Admission,
                                dimension,
                                required,
                                limit,
                            }
                            .into(),
                        ));
                    }
                }
                library
                    .charge(compute, 0)
                    .map_err(|error| failure(error.into()))?;
                let mut reservation = library
                    .memory_budget()
                    .reserve_capacity(envelope)
                    .map_err(|error| failure(error.into()))?;
                let mut owned = String::new();
                owned
                    .try_reserve_exact(text.len())
                    .map_err(|_| invalid("captured cell output allocation"))?;
                if owned.capacity() != text.len() {
                    return Err(invalid("captured cell exceeds admitted output capacity"));
                }
                owned.push_str(text);
                let canonical = SnapshotCanonicalizationBudget::new(finalization);
                let value = ValueDraft {
                    schema: descriptor.plan.values()[output.get() as usize].schema(),
                    shape_values: Box::new([]),
                    data: ValueDataDraft::String(owned),
                }
                .finalize(
                    &SnapshotValidationContext::with_shared_schemas(&descriptor.schemas)
                        .with_canonicalization_budget(&canonical),
                )
                .map_err(NativeProgramExecutionError::Snapshot)?
                .into_memory_budget(&mut reservation)
                .map_err(|error| failure(error.into()))?;
                self.arena
                    .put(output.get(), &value, &mut self.control.work)?;
                Ok(true)
            })
            .map_err(|error| failure(error.into()))?
    }
}

#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
pub(super) fn capture_source_coexistence(
    state: &OwnedState<'_>,
    skip: usize,
    memory: &ManagedMemoryBudget,
) -> Result<mech_core::ResourceDemand, NativeProgramExecutionError> {
    let mut prior = state
        .collections
        .as_ref()
        .map_or(Ok(mech_core::ResourceDemand::default()), |collections| {
            collections.retained_demand()
        })?;
    let lines = state.line_matrices.retained_demand(Some(skip), memory)?;
    prior.turn_peak_bytes = prior
        .turn_peak_bytes
        .checked_add(lines.turn_peak_bytes)
        .ok_or_else(|| invalid("capture source coexistence bytes overflow"))?;
    prior.retained_nodes = prior
        .retained_nodes
        .checked_add(lines.retained_nodes)
        .ok_or_else(|| invalid("capture source coexistence nodes overflow"))?;
    Ok(prior)
}

#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
impl Prepared<'_> {
    pub(super) fn install_final_line_count(
        &mut self,
        count: u64,
    ) -> Result<(), NativeProgramExecutionError> {
        let region = self
            .descriptor
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("final count without selected line frontier"))?;
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        let matrix = region.frontier.read().count().lines().matrix();
        let ordinal = self.descriptor.line_matrices.matrices[matrix.get() as usize]
            .ok_or_else(|| invalid("final count lost its selected line producer"))?;
        let actual = if let Some(view) = &state.line_matrices.views[ordinal] {
            view.count() as u64
        } else {
            let value = state.arena.copy_current(matrix.get(), &mut state.work)?;
            let ValueData::Matrix(matrix) = value.data() else {
                return Err(invalid("final count has no admitted line matrix"));
            };
            matrix.elements().len() as u64
        };
        if actual != count {
            return Err(invalid("final count differs from admitted source geometry"));
        }
        let counts = region.frontier.read().count().counts();
        state
            .work
            .charge(NativeExecutionPhase::Setup, amount(counts.len(), 16, 128)?)?;
        let first = self.descriptor.plan.transport_input_count();
        for value in counts {
            self.storage.values_mut()[first + value.get() as usize] = actual;
        }
        Ok(())
    }

    pub(super) fn begin_line_capture(
        &mut self,
        input_limit: u64,
        fragment: usize,
    ) -> Result<NativeTextLineChunks, NativeProgramExecutionError> {
        let descriptor = self.descriptor;
        let region = descriptor
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("capture lacks compiled retention proof"))?;
        let source = region.frontier.read().count().lines();
        if !self.activated
            || self.prefix.is_some()
            || descriptor.plan.transport_input_count() != 1
            || descriptor.plan.transport_input_value(0) != source.source()
            || source.lengths().len() != 1
            || descriptor.plan.blocks()[source.producer() as usize]
                .iteration()
                .is_some()
            || descriptor.plan.blocks()[source.lengths()[0] as usize]
                .iteration()
                .is_some()
        {
            return Err(invalid(
                "capture setup is outside supported original source visits",
            ));
        }
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        let ordinal = descriptor.line_matrices.matrices[source.matrix().get() as usize]
            .ok_or_else(|| invalid("capture lacks selected line owner slot"))?;
        if state.line_matrices.captured.is_some() || state.line_matrices.views[ordinal].is_some() {
            return Err(invalid("capture source already installed"));
        }
        let site = source.producer() as usize;
        let owner = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("capture source selection absent"))?
            .prepare_text_line_chunks(input_limit, fragment, self.limits.library, &state.library)
            .map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.plan.blocks()[site].origin(),
                error,
            })?;
        state.line_matrices.captured = Some(line_matrices::Captured {
            ordinal,
            produced: false,
            length: None,
        });
        Ok(owner)
    }

    pub(super) fn admit_line_capture(
        &mut self,
        owner: NativeTextLineChunks,
    ) -> Result<Rc<NativeTextLineMatrix>, NativeProgramExecutionError> {
        let descriptor = self.descriptor;
        let source = descriptor
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("capture lost compiled retention proof"))?
            .frontier
            .read()
            .count()
            .lines();
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        let captured = state
            .line_matrices
            .captured
            .as_ref()
            .ok_or_else(|| invalid("capture was not selected"))?;
        let length = captured
            .length
            .filter(|site| source.lengths() == [*site as u32])
            .ok_or_else(|| invalid("capture omitted original length visit"))?;
        if !captured.produced || state.line_matrices.views[captured.ordinal].is_some() {
            return Err(invalid(
                "capture omitted or repeated original producer visit",
            ));
        }
        let site = source.producer() as usize;
        let prior = capture_source_coexistence(state, captured.ordinal, &self.memory)?;
        let view = descriptor.calls[site]
            .as_ref()
            .unwrap()
            .text_line_matrix_from_capture(owner, self.limits.library, &state.library, prior)
            .map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.plan.blocks()[site].origin(),
                error,
            })?;
        descriptor.calls[length]
            .as_ref()
            .unwrap()
            .text_line_matrix_length(&view, self.limits.library, &state.library, prior)
            .map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.plan.blocks()[length].origin(),
                error,
            })?;
        let view = Rc::new(view);
        state.line_matrices.views[captured.ordinal] = Some(view.clone());
        Ok(view)
    }
}
