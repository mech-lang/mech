//! Synchronous invocation orchestration for this owner.
use super::*;
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact) fn captured_line_element_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<bool, NativeProgramExecutionError> {
        use mech_engine::native_artifact::NativeBorrowedStringAction;
        let descriptor = self.descriptor;
        let Some((_, NativeBorrowedStringAction::MatrixElement { source, output, .. })) =
            descriptor.metadata().borrowed_strings.action(site)
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
        let call = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("captured line element lost its selected factory"))?;
        use mech_core::snapshot::{
            SnapshotCanonicalizationBudget, prospective_string_value_footprint,
        };
        use mech_core::{
            MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase,
        };
        let failure = |error| NativeProgramExecutionError::Library {
            origin: descriptor.metadata().plan.blocks()[site].origin(),
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
        let library = self.state.library.clone();
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
                let envelope = amount(text.len(), 4, descriptor.metadata().scalar_bytes)?
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
                    schema: descriptor.metadata().plan.values()[output.get() as usize].schema(),
                    shape_values: Box::new([]),
                    data: ValueDataDraft::String(owned),
                }
                .finalize(
                    &SnapshotValidationContext::with_shared_schemas(&descriptor.metadata().schemas)
                        .with_canonicalization_budget(&canonical),
                )
                .map_err(NativeProgramExecutionError::Snapshot)?
                .into_memory_budget(&mut reservation)
                .map_err(|error| failure(error.into()))?;
                self.state
                    .arena
                    .put(output.get(), &value, &mut self.state.control.work)?;
                Ok(true)
            })
            .map_err(|error| failure(error.into()))?
    }
}
