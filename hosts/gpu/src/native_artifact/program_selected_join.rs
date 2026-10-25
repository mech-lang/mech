//! Preserve original range/selection/join visits without private Matrix payloads.
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
    NativeSelectedStringMatrix, NativeSelectedStringSpanSelector, NativeTextLineMatrix,
};
use std::rc::Rc;

#[path = "program_selected_join_map.rs"]
mod mapped;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program_runtime) use mapped::faults::{
    Control as JoinMapControl, Fault as JoinMapFault, Limit as JoinMapLimit,
    demand as join_map_demand, fired as join_map_fault_fired, started as join_map_started,
    with_control as with_join_map_control,
};
#[path = "program_selected_join_range.rs"]
mod range;

enum Phase {
    Range {
        lines: Rc<NativeTextLineMatrix>,
        selector: NativeSelectedStringSpanSelector,
    },
    Selected(NativeSelectedStringMatrix),
    Fusing(Box<mapped::Fusing>),
    Mapped(mapped::Mapped),
}
enum Completion {
    Value(Value),
    Private(State),
}

pub(super) struct State {
    phase: Phase,
    region: usize,
    parent: Option<u32>,
    // Retire genuine owners before releasing the admitted private metadata.
    reservation: ManagedMemoryReservation,
}
impl State {
    pub(super) fn caller_retained_nodes(&self) -> Result<u64, Error> {
        let nodes = match &self.phase {
            Phase::Range { lines, .. } => add(lines.count() as u64, 2)?,
            Phase::Selected(view) => view
                .retained_nodes_bound()
                .map_err(|_| invalid("selected join retained nodes changed"))?,
            Phase::Fusing(state) => add(
                state
                    .view
                    .retained_nodes_bound()
                    .map_err(|_| invalid("joined map retained nodes changed"))?,
                2,
            )?,
            Phase::Mapped(_) => 2,
        };
        add(nodes, self.reservation.capacity_bytes())
    }
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
    pub(super) fn selected_join_coexistence(
        &mut self,
        skip: Option<mech_engine::native_artifact::NativeMatchValue>,
    ) -> Result<ResourceDemand, Error> {
        let mut prior = self
            .collections
            .as_ref()
            .map_or(Ok(ResourceDemand::default()), |state| {
                state.retained_demand()
            })?;
        let lines = self.line_matrix_coexistence(skip)?;
        prior.turn_peak_bytes = add(prior.turn_peak_bytes, lines.turn_peak_bytes)?;
        prior.retained_nodes = add(prior.retained_nodes, lines.retained_nodes)?;
        Ok(prior)
    }

    pub(super) fn validate_selected_join_boundary(&self, site: usize) -> Result<(), Error> {
        let Some(state) = &self.selected_join else {
            return Ok(());
        };
        if matches!(state.phase, Phase::Fusing(_) | Phase::Mapped(_)) {
            return self.join_map_boundary(site, state);
        }
        let descriptor = self.control.descriptor;
        let region = descriptor
            .selected_joins
            .regions
            .get(state.region)
            .ok_or_else(|| invalid("selected join pending region is absent"))?;
        let expected = match state.phase {
            Phase::Range { .. } => region.selection(),
            Phase::Selected(_) => region.consumer(),
            Phase::Fusing(_) | Phase::Mapped(_) => unreachable!(),
        } as usize;
        let block = &descriptor.plan.blocks()[site];
        if block.iteration() != descriptor.plan.blocks()[region.selection() as usize].iteration()
            || self.control.current != state.parent
            || (site != expected
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
        range_tests::before_call(self, site, role, words)?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        if tests::before_call(self, site, role)? {
            return Ok(true);
        }
        #[cfg(all(test, feature = "canonical-native-program"))]
        mapped::faults::before_join(self, role);
        if !matches!(role, Site::Range(_)) && words != [0, 0] {
            return Err(invalid(
                "selected join requires its two managed operand words",
            ));
        }
        self.control.work.charge(NativeExecutionPhase::Setup, 512)?;
        let index = match role {
            Site::Range(index) | Site::Select(index) | Site::Join(index) => index,
        };
        let region = &descriptor.selected_joins.regions[index];
        let call = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("selected join lost its prepared call"))?;
        match role {
            Site::Range(_) => {
                let [start, end] = words else {
                    return Err(invalid("selected join range requires two scalar words"));
                };
                if self.selected_join.is_some() {
                    return Err(invalid("selected join range replayed before consumption"));
                }
                let proof = region
                    .range()
                    .ok_or_else(|| invalid("selected join range lacks its original proof"))?;
                if proof.producer() as usize != site {
                    return Err(invalid("selected join range belongs to another visit"));
                }
                let Some(lines) = self.line_matrix(region.source()).cloned() else {
                    // No private source exists: the original Range must create
                    // its genuine canonical selector for the ordinary path.
                    return Ok(false);
                };
                self.prepare_selected_join_range(site, index, lines, [*start, *end])?;
            }
            Site::Select(_) => {
                let lines = self.line_matrix(region.source()).cloned();
                let pending = self.selected_join.take();
                let (compact, reserved) = match pending {
                    Some(State {
                        phase:
                            Phase::Range {
                                lines: owner,
                                selector,
                            },
                        region: pending_region,
                        parent,
                        reservation,
                    }) if pending_region == index
                        && parent == self.control.current
                        && region.range().is_some()
                        && lines
                            .as_ref()
                            .is_some_and(|lines| Rc::ptr_eq(lines, &owner)) =>
                    {
                        (Some(selector), Some(reservation))
                    }
                    Some(_) => {
                        return Err(invalid("selected join selection has another pending phase"));
                    }
                    None if region.range().is_some() && lines.is_some() => {
                        return Err(invalid("selected join compact range is missing"));
                    }
                    None => (None, None),
                };
                if region.range().is_some()
                    && compact.is_none()
                    && self
                        .arena
                        .current_retained_bytes(region.selector().get(), &mut self.control.work)?
                        .is_none()
                {
                    return Err(invalid(
                        "selected join has neither compact nor canonical range",
                    ));
                }
                if lines.is_none() {
                    for (lane, value) in
                        [region.source(), region.selector()].into_iter().enumerate()
                    {
                        self.call_inputs.values_mut()[lane] = Some(
                            self.arena
                                .copy_current(value.get(), &mut self.control.work)?,
                        );
                    }
                    if !call.supports_selected_string_matrix(&self.call_inputs.values()[..2]) {
                        self.call_inputs.values_mut()[..2].fill(None);
                        return Ok(false);
                    }
                }
                let metadata = reserved.as_ref().map_or(
                    size_of::<State>() as u64,
                    ManagedMemoryReservation::capacity_bytes,
                );
                let mut prior = self.selected_join_coexistence(Some(region.source()))?;
                prior.turn_peak_bytes = add(prior.turn_peak_bytes, metadata)?;
                prior.retained_nodes = add(prior.retained_nodes, 1)?;
                require(
                    self,
                    site,
                    Dimension::TemporaryBytes,
                    prior.turn_peak_bytes,
                    self.limits.max_temporary_bytes,
                )?;
                let reservation = if let Some(reservation) = reserved {
                    reservation
                } else {
                    self.control
                        .work
                        .charge(NativeExecutionPhase::Setup, add(mul(metadata, 8)?, 256)?)?;
                    self.library
                        .memory_budget()
                        .reserve_capacity(metadata)
                        .map_err(|error| library(self, site, error.into()))?
                };
                let result = if let Some(lines) = lines {
                    if let Some(selector) = compact {
                        call.selected_text_line_matrix_range(
                            &lines,
                            &selector,
                            self.limits,
                            &self.library,
                            prior,
                        )
                    } else {
                        let selector = self
                            .arena
                            .copy_current(region.selector().get(), &mut self.control.work)?;
                        call.selected_text_line_matrix(
                            &lines,
                            &selector,
                            self.limits,
                            &self.library,
                            prior,
                        )
                    }
                } else {
                    call.selected_string_matrix(
                        &self.call_inputs.values()[..2],
                        self.limits,
                        &self.library,
                        prior,
                    )
                };
                self.call_inputs.values_mut()[..2].fill(None);
                let view = result.map_err(|error| library(self, site, error))?;
                // A previous ordinary-representation iteration may have left
                // its unobservable result here. Retire it only after the fresh
                // projection succeeds; never substitute it for this view.
                self.arena
                    .retire_current(region.selected().get(), &mut self.control.work)?;
                if let Some(captured) = &self.line_matrices.captured {
                    let lines = &descriptor.line_matrices.regions[captured.ordinal];
                    if lines.matrix() == region.source()
                        && lines.selections() == [site as u32]
                        && self.line_window.is_none()
                        && self.prefix.as_ref().is_some_and(|prefix| prefix.input.eof)
                    {
                        // The complete-use proof has one final selection, and
                        // the scanner's EOF checkpoint already ended its loan.
                        // Transfer the remaining owner to this selected join.
                        self.line_matrices.views[captured.ordinal] = None;
                    }
                }
                self.selected_join = Some(State {
                    phase: Phase::Selected(view),
                    region: index,
                    parent: self.control.current,
                    reservation,
                });
                #[cfg(all(test, feature = "canonical-native-program"))]
                range_tests::after_selection(self, index, compact.is_some());
                // No Matrix is installed in the semantic selection-output slot.
            }
            Site::Join(_) => {
                let Some(state) = self.selected_join.take() else {
                    // Fallback is possible only with a real canonical result.
                    // Missing/replayed private state must never fall through
                    // to a call needing an intentionally absent Matrix.
                    if self
                        .arena
                        .current_retained_bytes(region.selected().get(), &mut self.control.work)?
                        .is_none()
                    {
                        return Err(invalid("selected join consumer has no selected owner"));
                    }
                    return Ok(false);
                };
                if state.region != index
                    || state.parent != self.control.current
                    || !matches!(state.phase, Phase::Selected(_))
                {
                    return Err(invalid("selected join consumer has another projection"));
                }
                let separator = self
                    .arena
                    .copy_current(region.separator().get(), &mut self.control.work)?;
                match self.finish_selected_join(site, state, &separator)? {
                    Completion::Value(value) => {
                        self.arena
                            .put(region.output().get(), &value, &mut self.control.work)?;
                    }
                    Completion::Private(state) => {
                        self.selected_join = Some(state);
                    }
                }
            }
        }
        Ok(true)
    }

    fn finish_selected_join(
        &mut self,
        site: usize,
        state: State,
        separator: &Value,
    ) -> Result<Completion, Error> {
        let State {
            phase,
            region: region_index,
            parent,
            reservation: state_reservation,
        } = state;
        let Phase::Selected(view) = phase else {
            return Err(invalid("selected join consumer precedes selection"));
        };
        let source = self.control.descriptor.selected_joins.regions[region_index].source();
        let prior = self.selected_join_coexistence(Some(source))?;
        let compute_limit = self
            .limits
            .max_compute_work
            .ok_or_else(|| invalid("selected join compute limit"))?;
        let comparison_limit = self
            .limits
            .max_comparison_work
            .ok_or_else(|| invalid("selected join comparison limit"))?;
        let account = self.library.clone();
        account
            .with_call_limits(compute_limit, comparison_limit, || {
                self.library
                    .charge(512, 0)
                    .map_err(|e| library(self, site, e.into()))?;
                let memory = self.library.memory_budget();
                let descriptor = self.control.descriptor;
                let region = &descriptor.selected_joins.regions[region_index];
                let call = descriptor.calls[site]
                    .as_ref()
                    .ok_or_else(|| invalid("selected join consumer call"))?;
                let ValueData::String(text_separator) = separator.data() else {
                    return Err(invalid("selected join separator is not a String"));
                };
                let retained = view
                    .retained_bytes(&memory)
                    .map_err(|e| library(self, site, e))?;
                let separator_bytes = separator
                    .memory_budget_retained_bytes(&memory)
                    .ok_or_else(|| library(self, site, InvokeError::UnadmittedValue))?;
                let payload = add(
                    view.payload_bytes(),
                    mul(
                        view.count().saturating_sub(1) as u64,
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
                    add(mul(view.count() as u64, 128)?, 1024)?,
                )?;
                let peak = add(
                    add(
                        add(
                            add(prior.turn_peak_bytes, state_reservation.capacity_bytes())?,
                            retained,
                        )?,
                        separator_bytes,
                    )?,
                    envelope,
                )?;
                let nodes = add(
                    add(
                        prior.retained_nodes,
                        view.retained_nodes_bound()
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
                #[cfg(all(test, feature = "canonical-native-program"))]
                let _composition_pressure = mapped::faults::pressure(self)?;
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
                if let Some(prepared) =
                    self.prepare_join_map(site, region_index, &view, separator, &text, compute)?
                {
                    let fusing =
                        mapped::Fusing::new(prepared, view, text, reservation, finalization)?;
                    return Ok(Completion::Private(State {
                        phase: Phase::Fusing(fusing),
                        region: region_index,
                        parent,
                        reservation: state_reservation,
                    }));
                }
                call.selected_matrix_join_append_consuming(view, separator, &mut text, &memory)
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
                .map(Completion::Value)
            })
            .map_err(|e| library(self, site, e.into()))?
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_selected_join_tests.rs"]
mod tests;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_selected_join_range_tests.rs"]
mod range_tests;
