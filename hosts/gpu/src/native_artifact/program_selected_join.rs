//! Preserve the two original selected visits while omitting their private Matrix.
use super::{Context, NativeProgramExecutionError as Error, invalid};
use crate::native_artifact::{NativeExecutionPhase, program_selected_join_layout::Site};
use mech_core::{
    ManagedMemoryReservation, MemoryBudgetDimension as Dimension, ResidentKernelError,
    ResidentResourcePhase, ResourceDemand, Value, ValueData,
    snapshot::{
        SnapshotCanonicalizationBudget, SnapshotValidationContext, ValueDataDraft, ValueDraft,
        prospective_string_value_footprint,
    },
};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError as InvokeError, NativeIterationAction,
    NativeIterationTerminator, NativeMatchAction, NativeMatchTerminator,
    NativeSelectedStringMatrix,
};

pub(super) struct State {
    view: NativeSelectedStringMatrix,
    region: usize,
    parent: Option<u32>,
    // Retire genuine owners before releasing the admitted private metadata.
    reservation: ManagedMemoryReservation,
}

fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b)
        .ok_or_else(|| invalid("selected join demand overflow"))
}
fn mul(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_mul(b)
        .ok_or_else(|| invalid("selected join demand overflow"))
}
fn library(ctx: &Context<'_, '_>, site: usize, error: InvokeError) -> Error {
    Error::Library {
        origin: ctx.control.descriptor.plan.blocks()[site].origin(),
        error,
    }
}
fn require(
    ctx: &Context<'_, '_>,
    site: usize,
    dimension: Dimension,
    required: u64,
    limit: Option<u64>,
) -> Result<(), Error> {
    let limit = limit.ok_or_else(|| invalid("selected join requires a finite limit"))?;
    if required > limit {
        return Err(library(
            ctx,
            site,
            ResidentKernelError::ResourceLimit {
                phase: ResidentResourcePhase::Admission,
                dimension,
                required,
                limit,
            }
            .into(),
        ));
    }
    Ok(())
}

impl Context<'_, '_> {
    fn selected_join_coexistence(&self) -> Result<ResourceDemand, Error> {
        self.collections
            .as_ref()
            .map_or(Ok(ResourceDemand::default()), |state| {
                state.retained_demand()
            })
    }

    pub(super) fn validate_selected_join_boundary(&self, site: usize) -> Result<(), Error> {
        let Some(state) = &self.selected_join else {
            return Ok(());
        };
        let descriptor = self.control.descriptor;
        let region = &descriptor.selected_joins.regions[state.region];
        let block = &descriptor.plan.blocks()[site];
        if block.iteration() != descriptor.plan.blocks()[region.selection() as usize].iteration()
            || self.control.current != state.parent
            || (site != region.consumer() as usize
                && !matches!(
                    (block.action(), block.terminator()),
                    (
                        NativeIterationAction::Scalar(NativeMatchAction::None),
                        NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(_))
                    )
                ))
        {
            return Err(invalid(
                "selected join owners escaped their proved adjacent visits",
            ));
        }
        Ok(())
    }

    /// The outer helper already checked the exact current visit and ABI lane.
    /// False selects the ordinary maintained calls, never a different executor.
    pub(super) fn selected_join_call(&mut self, site: usize, words: &[u64]) -> Result<bool, Error> {
        let descriptor = self.control.descriptor;
        let Some(role) = descriptor.selected_joins.sites[site] else {
            return Ok(false);
        };
        #[cfg(all(test, feature = "canonical-native-program"))]
        if tests::before_call(self, site, role)? {
            return Ok(true);
        }
        if words != [0, 0] {
            return Err(invalid(
                "selected join requires its two managed operand words",
            ));
        }
        self.control.work.charge(NativeExecutionPhase::Setup, 512)?;
        let index = match role {
            Site::Select(index) | Site::Join(index) => index,
        };
        let region = &descriptor.selected_joins.regions[index];
        let call = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("selected join lost its prepared call"))?;
        match role {
            Site::Select(_) => {
                if self.selected_join.is_some() {
                    return Err(invalid("selected join projection was not consumed"));
                }
                for (lane, value) in [region.source(), region.selector()].into_iter().enumerate() {
                    self.call_inputs.values_mut()[lane] = Some(
                        self.arena
                            .copy_current(value.get(), &mut self.control.work)?,
                    );
                }
                if !call.supports_selected_string_matrix(&self.call_inputs.values()[..2]) {
                    self.call_inputs.values_mut()[..2].fill(None);
                    return Ok(false);
                }
                let metadata = size_of::<State>() as u64;
                let mut prior = self.selected_join_coexistence()?;
                prior.turn_peak_bytes = add(prior.turn_peak_bytes, metadata)?;
                prior.retained_nodes = add(prior.retained_nodes, 1)?;
                require(
                    self,
                    site,
                    Dimension::TemporaryBytes,
                    prior.turn_peak_bytes,
                    self.limits.max_temporary_bytes,
                )?;
                self.control
                    .work
                    .charge(NativeExecutionPhase::Setup, add(mul(metadata, 8)?, 256)?)?;
                let reservation = self
                    .library
                    .memory_budget()
                    .reserve_capacity(metadata)
                    .map_err(|error| library(self, site, error.into()))?;
                let result = call.selected_string_matrix(
                    &self.call_inputs.values()[..2],
                    self.limits,
                    &self.library,
                    prior,
                );
                self.call_inputs.values_mut()[..2].fill(None);
                let view = result.map_err(|error| library(self, site, error))?;
                // A previous ordinary-representation iteration may have left
                // its unobservable result here. Retire it only after the fresh
                // projection succeeds; never substitute it for this view.
                self.arena
                    .retire_current(region.selected().get(), &mut self.control.work)?;
                self.selected_join = Some(State {
                    view,
                    region: index,
                    parent: self.control.current,
                    reservation,
                });
                // No Matrix is installed in the semantic selection-output slot.
            }
            Site::Join(_) => {
                let Some(state) = self.selected_join.take() else {
                    return Ok(false); // Producer used the ordinary representation.
                };
                if state.region != index || state.parent != self.control.current {
                    return Err(invalid("selected join consumer has another projection"));
                }
                let separator = self
                    .arena
                    .copy_current(region.separator().get(), &mut self.control.work)?;
                let value = self.finish_selected_join(site, &state, &separator)?;
                self.arena
                    .put(region.output().get(), &value, &mut self.control.work)?;
                drop(state);
            }
        }
        Ok(true)
    }

    fn finish_selected_join(
        &self,
        site: usize,
        state: &State,
        separator: &Value,
    ) -> Result<Value, Error> {
        let compute_limit = self
            .limits
            .max_compute_work
            .ok_or_else(|| invalid("selected join compute limit"))?;
        let comparison_limit = self
            .limits
            .max_comparison_work
            .ok_or_else(|| invalid("selected join comparison limit"))?;
        self.library
            .with_call_limits(compute_limit, comparison_limit, || {
                self.library
                    .charge(512, 0)
                    .map_err(|e| library(self, site, e.into()))?;
                let memory = self.library.memory_budget();
                let descriptor = self.control.descriptor;
                let region = &descriptor.selected_joins.regions[state.region];
                let call = descriptor.calls[site]
                    .as_ref()
                    .ok_or_else(|| invalid("selected join consumer call"))?;
                let ValueData::String(text_separator) = separator.data() else {
                    return Err(invalid("selected join separator is not a String"));
                };
                let retained = state
                    .view
                    .retained_bytes(&memory)
                    .map_err(|e| library(self, site, e))?;
                let separator_bytes = separator
                    .memory_budget_retained_bytes(&memory)
                    .ok_or_else(|| library(self, site, InvokeError::UnadmittedValue))?;
                let payload = add(
                    state.view.payload_bytes(),
                    mul(
                        state.view.count().saturating_sub(1) as u64,
                        text_separator.len() as u64,
                    )?,
                )?;
                let footprint = prospective_string_value_footprint(0, false, 1, payload)
                    .map_err(|_| invalid("selected join output footprint"))?;
                // Covers the actual target and canonical construction/import together,
                // not a nonexistent intermediate Matrix or a credit for future drops.
                let envelope = add(descriptor.scalar_bytes, add(mul(payload, 4)?, 4096)?)?;
                let finalization = add(mul(payload, 8)?, 32)?;
                let compute = add(
                    add(mul(envelope, 16)?, finalization)?,
                    add(mul(state.view.count() as u64, 128)?, 1024)?,
                )?;
                let prior = self.selected_join_coexistence()?;
                let peak = add(
                    add(
                        add(
                            add(prior.turn_peak_bytes, state.reservation.capacity_bytes())?,
                            retained,
                        )?,
                        separator_bytes,
                    )?,
                    envelope,
                )?;
                let nodes = add(
                    add(
                        prior.retained_nodes,
                        state
                            .view
                            .retained_nodes_bound()
                            .map_err(|e| library(self, site, e))?,
                    )?,
                    8,
                )?;
                #[cfg(all(test, feature = "canonical-native-program"))]
                let _pressure = tests::observe(
                    self,
                    site,
                    tests::Demand {
                        temporary: peak,
                        output: footprint.retained_bytes,
                        cloned: mul(payload, 2)?,
                        nodes,
                        compute: add(compute, 512)?,
                        compute_prefix: self
                            .library
                            .used()
                            .0
                            .checked_sub(512)
                            .ok_or_else(|| invalid("selected join lost its paid header"))?,
                        envelope,
                        selection_slot_empty: self
                            .arena
                            .test_current_value(region.selected().get())
                            .is_none(),
                    },
                )?;
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
                        mul(payload, 2)?,
                        self.limits.max_cloned_bytes,
                    ),
                    (
                        Dimension::TemporaryBytes,
                        peak,
                        self.limits.max_temporary_bytes,
                    ),
                    (
                        Dimension::RetainedNodes,
                        nodes,
                        self.limits.max_retained_nodes,
                    ),
                    (
                        Dimension::ComparisonWork,
                        0,
                        self.limits.max_comparison_work,
                    ),
                ] {
                    require(self, site, dimension, required, limit)?;
                }
                self.library
                    .charge(compute, 0)
                    .map_err(|e| library(self, site, e.into()))?;
                let mut reservation = memory
                    .reserve_capacity(envelope)
                    .map_err(|e| library(self, site, e.into()))?;
                let mut text = String::new();
                let capacity = usize::try_from(payload)
                    .map_err(|_| invalid("selected join output address overflow"))?;
                text.try_reserve_exact(capacity)
                    .map_err(|_| invalid("selected join output allocation"))?;
                if text.capacity() != capacity {
                    return Err(invalid("selected join output exceeds admitted capacity"));
                }
                call.selected_matrix_join_append(&state.view, separator, &mut text, &memory)
                    .map_err(|e| library(self, site, e))?;
                let canonical = SnapshotCanonicalizationBudget::new(finalization);
                let output = &descriptor.plan.values()[region.output().get() as usize];
                ValueDraft {
                    schema: output.schema(),
                    shape_values: Box::new([]),
                    data: ValueDataDraft::String(text),
                }
                .finalize(
                    &SnapshotValidationContext::with_shared_schemas(&descriptor.schemas)
                        .with_canonicalization_budget(&canonical),
                )
                .map_err(Error::Snapshot)?
                .into_memory_budget(&mut reservation)
                .map_err(|e| library(self, site, e.into()))
            })
            .map_err(|e| library(self, site, e.into()))?
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_selected_join_tests.rs"]
mod tests;
