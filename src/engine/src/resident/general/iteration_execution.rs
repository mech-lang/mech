//! Bounded-frame execution of a genuine counted lexical application.
//!
//! The private carrier and body locals never alias the enclosing result. Each
//! complete body candidate is checked and admitted before replacing the
//! carrier; only the final carrier reaches the ordinary result region.
use super::*;
use crate::resident::general::{ActivatedIterationCount, ActivatedIterationNode, live};
use mech_core::{CurrentMemoryFootprint, ResidentPortLayout};

#[path = "iteration_elements.rs"]
mod elements;

fn checked_add(left: u64, right: u64) -> Result<u64, ResidentKernelError> {
    left.checked_add(right)
        .ok_or(ResidentKernelError::InvalidShape)
}

fn checked_mul(left: u64, right: u64) -> Result<u64, ResidentKernelError> {
    left.checked_mul(right)
        .ok_or(ResidentKernelError::InvalidShape)
}

fn admit(
    cost: impl FnOnce() -> Result<budget::KernelCostEstimate, ResidentKernelError>,
) -> Result<(), ResidentKernelError> {
    budget::PreparedKernel::new((), cost()?)
        .admit_control()?
        .into_plan();
    Ok(())
}

impl ReactiveInstance {
    pub(super) fn execute_iteration_with_live_demand(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        live_bytes: u64,
        live_nodes: u64,
    ) -> Result<bool, ResidentExecutionError> {
        // A carrier replacement can change the selected String each visit.
        // Preserve outer lexical backing but never reuse an enclosing index
        // across this unproved mutable carrier/control boundary.
        let _text_barrier = crate::resident::text_index::LookupBarrier::enter();
        let ActivatedTurnStep::Iteration(control) = &self.plan.steps[node_index.get() as usize]
        else {
            unreachable!("iteration dispatch")
        };
        let control = std::sync::Arc::clone(control);
        let node = control.artifact_node;
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let current = if control.write.storage == ResidentStorageClass::Constant {
            self.activation.read(control.write.region)
        } else {
            self.workspace.scratch.read(control.write.region)
        };
        let call = self
            .plan
            .memory_plan
            .call_for_node(control.budget_node)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let mut facts = live::facts(call, control.budget_node, &[], current, &self.plan.schemas)
            .map_err(|error| fail(budget::memory_plan_kernel_error(error)))?;
        let measurement_work = facts.additional_demand.work;
        facts.additional_demand.turn_peak_bytes = live_bytes;
        facts.additional_demand.retained_nodes = live_nodes;
        let turn_plan = crate::memory_planner::plan_current_resident_turn(
            &self.plan.memory_plan,
            control.budget_node,
            &facts,
        )
        .map_err(|error| fail(budget::memory_plan_kernel_error(error)))?;
        if let Some(violation) = turn_plan.budget_violations.first() {
            return Err(fail(budget::resource_budget_error(violation)));
        }
        budget::with_control_work_budget(|| {
            budget::with_resident_turn_plan(turn_plan, || {
                let result = (|| {
                    admit(|| {
                        Ok(budget::resident_cost! {
                            compute_work: measurement_work.compute,
                            comparison_work: measurement_work.comparison,
                            ..budget::KernelCostEstimate::default()
                        })
                    })
                    .map_err(fail)?;
                    // The old enclosing result survives every iteration and
                    // every failure. Measure it before staging the first seed.
                    let old = if control.write.storage == ResidentStorageClass::Constant {
                        self.activation.read(control.write.region)
                    } else {
                        self.workspace.scratch.read(control.write.region)
                    };
                    let mut meter = budget::ResidentBudgetMeter::default();
                    let retained =
                        resident_frame_value_footprint(old, &self.plan.schemas, &mut meter)
                            .map_err(fail)?;
                    admit(|| {
                        Ok(budget::resident_cost! {
                            compute_work: meter.estimate().compute_work()
                                .checked_add(control.locals.len() as u64)
                                .and_then(|work| work.checked_add(1))
                                .ok_or(ResidentKernelError::InvalidShape)?,
                            comparison_work: meter.estimate().comparison_work(),
                            temporary_bytes: retained.0,
                            retained_nodes: retained.1,
                            ..budget::KernelCostEstimate::default()
                        })
                    })
                    .map_err(fail)?;
                    let body_live = (
                        checked_add(live_bytes, retained.0).map_err(fail)?,
                        checked_add(live_nodes, retained.1).map_err(fail)?,
                    );
                    if let ActivatedIterationCount::Elements { read, layout } = &control.count {
                        self.execute_element_iterations(
                            &control,
                            *read,
                            layout,
                            (before_epoch, working_epoch),
                            probe,
                            body_live,
                        )?;
                        set_bit(
                            &mut self.workspace.initialized_output_bits,
                            node_index.get() as usize,
                        );
                        return Ok(true);
                    }
                    self.iteration_copy_boundary(
                        node,
                        control.carrier_memory_node,
                        control.seed,
                        &control.seed_layout,
                        control.carrier,
                        working_epoch,
                        body_live,
                    )?;
                    let ActivatedIterationCount::Literal(count) = control.count else {
                        unreachable!()
                    };
                    self.execute_iteration_body(
                        &control,
                        count,
                        (before_epoch, working_epoch),
                        probe,
                        body_live,
                    )?;
                    let carrier_layout = super::super::slot_port_layout(
                        &self.plan.slots[control.carrier.slot.get() as usize],
                    );
                    self.iteration_copy_boundary(
                        node,
                        control.budget_node,
                        ResidentReadLocation::Scratch(control.carrier.region),
                        &carrier_layout,
                        control.write,
                        working_epoch,
                        body_live,
                    )?;
                    set_bit(
                        &mut self.workspace.initialized_output_bits,
                        node_index.get() as usize,
                    );
                    Ok(true)
                })();
                for region in control.locals.iter().copied() {
                    self.workspace.scratch.discard_payload_write(region);
                }
                result
            })
        })
    }

    fn execute_iteration_body(
        &mut self,
        control: &ActivatedIterationNode,
        count: u64,
        (before_epoch, working_epoch): (InstanceEpoch, InstanceEpoch),
        probe: &mut ResidentStructuralProbe,
        body_live: (u64, u64),
    ) -> Result<(), ResidentExecutionError> {
        let node = control.artifact_node;
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let body = control
            .body
            .as_ref()
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
        let mut completed = 0_u64;
        while completed < count {
            budget::charge_planning_step()?;
            // Identity/empty bodies still consume one visit. Do
            // not reserve count*cost in advance: an earlier checked
            // operation must precede a later work denial.
            admit(|| {
                Ok(budget::resident_cost! {
                    compute_work: 1,
                    ..budget::KernelCostEstimate::default()
                })
            })
            .map_err(fail)?;
            let mut local_meter = budget::ResidentBudgetMeter::default();
            let mut block_live = ControlBlockLiveFootprint::default();
            for step in body.steps.iter() {
                self.execute_control_step_with_live_demand(
                    node,
                    body,
                    step,
                    &mut block_live,
                    &mut local_meter,
                    (before_epoch, working_epoch),
                    probe,
                    body_live,
                )?;
            }
            // Every scheduled local, including unused checked
            // nodes, remains live until this boundary is admitted.
            let retained_locals = self
                .resident_local_footprint(
                    body.locals.iter().copied(),
                    &self.plan.schemas,
                    &mut local_meter,
                )
                .map_err(fail)?;
            // This borrowed planning walk includes locals that
            // are not the yield. Charge its complete estimate in a
            // fresh visit scope before staging the next carrier.
            admit(|| Ok(local_meter.estimate())).map_err(fail)?;
            self.iteration_copy_boundary(
                node,
                control.carrier_memory_node,
                body.yield_value,
                &body.yield_layout,
                control.carrier,
                working_epoch,
                (
                    checked_add(body_live.0, retained_locals.retained_bytes).map_err(fail)?,
                    checked_add(body_live.1, retained_locals.node_count).map_err(fail)?,
                ),
            )?;
            admit(|| {
                Ok(budget::resident_cost! {
                    compute_work: control.locals.len() as u64,
                    ..budget::KernelCostEstimate::default()
                })
            })
            .map_err(fail)?;
            for region in control.locals.iter().copied() {
                if region != control.carrier.region {
                    self.workspace.scratch.discard_payload_write(region);
                }
            }
            completed += 1; // completed<count<=u64::MAX proves this addition.
        }
        Ok(())
    }

    fn iteration_copy_boundary(
        &mut self,
        node: NodeId,
        memory_node: NodeId,
        source_location: ResidentReadLocation,
        source_layout: &ResidentPortLayout,
        target: super::super::ResidentWriteLocation,
        epoch: InstanceEpoch,
        live: (u64, u64),
    ) -> Result<(), ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let current = if target.storage == ResidentStorageClass::Constant {
            self.activation.read(target.region)
        } else {
            self.workspace.scratch.read(target.region)
        };
        let call = self
            .plan
            .memory_plan
            .call_for_node(memory_node)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let facts = live::facts(call, memory_node, &[], current, &self.plan.schemas)
            .map_err(|error| fail(budget::memory_plan_kernel_error(error)))?;
        let measurement_work = facts.additional_demand.work;
        let plan = crate::memory_planner::plan_current_resident_turn(
            &self.plan.memory_plan,
            memory_node,
            &facts,
        )
        .map_err(|error| fail(budget::memory_plan_kernel_error(error)))?;
        if let Some(violation) = plan.budget_violations.first() {
            return Err(fail(budget::resource_budget_error(violation)));
        }
        budget::with_resident_turn_plan(plan, || {
            admit(|| {
                Ok(budget::resident_cost! {
                    compute_work: measurement_work.compute,
                    comparison_work: measurement_work.comparison,
                    ..budget::KernelCostEstimate::default()
                })
            })
            .map_err(fail)?;
            self.iteration_copy_boundary_planned(
                node,
                source_location,
                source_layout,
                target,
                epoch,
                live,
            )
        })
    }

    fn iteration_copy_boundary_planned(
        &mut self,
        node: NodeId,
        source_location: ResidentReadLocation,
        source_layout: &ResidentPortLayout,
        target: super::super::ResidentWriteLocation,
        epoch: InstanceEpoch,
        live: (u64, u64),
    ) -> Result<(), ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let source = self
            .read_location(source_location, epoch)
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
        let target_slot = &self.plan.slots[target.slot.get() as usize];
        let source_schema = self
            .plan
            .schemas
            .get(source_layout.schema_id)
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
        let target_schema = self
            .plan
            .schemas
            .get(target_slot.schema)
            .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?;
        let metadata = checked_add(
            self.plan
                .schemas
                .entry(source_layout.schema_id)
                .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?
                .canonical_bytes()
                .len() as u64,
            self.plan
                .schemas
                .entry(target_slot.schema)
                .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?
                .canonical_bytes()
                .len() as u64,
        )
        .and_then(|work| checked_add(work, 16))
        .map_err(fail)?;
        // All supported shapes are scalar or flat rank two. Fund the complete
        // schema/shape metadata comparisons before resolving any dimensions.
        admit(|| {
            Ok(budget::resident_cost! {
                comparison_work: metadata,
                compute_work: metadata,
                temporary_bytes: checked_mul(metadata, 8)?,
                ..budget::KernelCostEstimate::default()
            })
        })
        .map_err(fail)?;
        if !crate::artifact::iteration_carrier_compatible(source_schema, target_schema) {
            return Err(fail(ResidentKernelError::InvalidShape));
        }
        let source_shape = match source {
            ResidentValueRef::Snapshot([Some(value)]) => {
                if value.schema() != source_layout.schema_id
                    || value.schema_key() != source_layout.schema_key
                {
                    return Err(fail(ResidentKernelError::InvalidInput));
                }
                value
                    .validate_against(&self.plan.schemas)
                    .map_err(|_| fail(ResidentKernelError::InvalidInput))?;
                value.shape()
            }
            ResidentValueRef::Snapshot(_) => return Err(fail(ResidentKernelError::InvalidInput)),
            _ => &source_layout.shape_instance,
        };
        let (shape, elements) = match source_schema.body() {
            mech_core::SchemaBody::Matrix { dimensions, .. } => {
                let extents = [
                    source_shape
                        .resolve_dimension(&dimensions[0])
                        .map_err(|_| fail(ResidentKernelError::InvalidShape))?,
                    source_shape
                        .resolve_dimension(&dimensions[1])
                        .map_err(|_| fail(ResidentKernelError::InvalidShape))?,
                ];
                (
                    mech_core::shape_for_resolved_extents(target_schema, &extents)
                        .map_err(|_| fail(ResidentKernelError::InvalidShape))?,
                    checked_mul(extents[0], extents[1]).map_err(fail)?,
                )
            }
            _ => (
                target_schema
                    .instantiate_shape(Box::new([]))
                    .map_err(|_| fail(ResidentKernelError::InvalidShape))?,
                1,
            ),
        };
        if (target_slot.activation_fixed_shape && target_slot.shape != shape)
            || !mech_core::shape_change_allowed(target_schema, &target_slot.shape, &shape)
        {
            return Err(fail(ResidentKernelError::InvalidShape));
        }
        let exact_snapshot = target.region.kind == ResidentValueKind::Snapshot
            && matches!(source, ResidentValueRef::Snapshot([Some(value)])
                if value.schema() == target_slot.schema
                    && value.schema_key() == target_slot.schema_key
                    && value.shape() == &shape);
        let mut meter = budget::ResidentBudgetMeter::default();
        let (source_footprint, dense_cost) = match source {
            ResidentValueRef::Snapshot([Some(value)]) => {
                let owner = value.schemas();
                let owner = owner.as_deref().unwrap_or(&self.plan.schemas);
                let footprint = if exact_snapshot {
                    // Exact-schema/shape transfer shares the immutable root.
                    // Retention observes metadata without scanning/copying bytes.
                    budget::measure_borrowed_canonical_value_footprint(&mut meter, value, owner)
                } else {
                    budget::measure_canonical_value_footprint(&mut meter, value, owner)
                }
                .map_err(fail)?;
                let schema = owner
                    .get(value.schema())
                    .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                if !exact_snapshot {
                    budget::preflight_canonical_data_finalization(
                        &mut meter,
                        schema.body(),
                        value.data(),
                    )
                    .map_err(fail)?;
                }
                (footprint, None)
            }
            _ => {
                // The dense String bound scans headers only after this exact
                // per-cell precharge. The materializer subsequently copies all
                // UTF-8 bytes under the complete byte-aware admission below.
                admit(|| {
                    Ok(budget::resident_cost! {
                        compute_work: source.len() as u64,
                        ..budget::KernelCostEstimate::default()
                    })
                })
                .map_err(fail)?;
                let cost = crate::resident::numeric::resident_value_snapshot_materialization_cost(
                    &self.plan.schemas,
                    source_layout.schema_id,
                    source_shape,
                    source,
                )
                .map_err(fail)?;
                (
                    ValueFootprint {
                        encoded_bytes: cost.persistent_bytes,
                        retained_bytes: cost.persistent_bytes,
                        node_count: cost.retained_nodes,
                    },
                    Some(cost),
                )
            }
        };
        let payload = source_footprint.retained_bytes;
        let observed = meter.estimate();
        let dense_temporary = dense_cost.map_or(0, |cost| cost.temporary_bytes);
        let dense_schema_copy =
            if dense_cost.is_some() && target.region.kind == ResidentValueKind::Snapshot {
                self.plan
                    .schemas
                    .clone_allocation_bound_bytes()
                    .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?
            } else {
                0
            };
        let shape_parameters = shape.parameter_values().len() as u64;
        let footprint = CurrentMemoryFootprint {
            logical_elements: elements,
            payload_bytes: payload,
            encoded_bytes: source_footprint.encoded_bytes,
            retained_nodes: source_footprint.node_count,
            shape_parameter_count: shape_parameters,
            ..CurrentMemoryFootprint::default()
        };
        let (persistent, workspace) = if exact_snapshot {
            // A finalized root with exact schema/shape is shared, not rebuilt.
            // Retained payload is still reported in the logical output bound;
            // only the small owning handle is new transient storage.
            (
                checked_add(payload, Value::memory_budget_claim_metadata_bytes()).map_err(fail)?,
                core::mem::size_of::<Value>() as u64,
            )
        } else {
            let draft = mech_core::canonical_snapshot_draft_bytes(footprint)
                .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
            let finalization = mech_core::canonical_snapshot_finalization_bytes(footprint)
                .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
            (
                checked_add(finalization, Value::memory_budget_claim_metadata_bytes())
                    .map_err(fail)?,
                checked_add(draft, finalization)
                    .and_then(|bytes| checked_add(bytes, dense_temporary))
                    .and_then(|bytes| checked_add(bytes, dense_schema_copy))
                    .map_err(fail)?,
            )
        };
        let work = checked_add(
            if exact_snapshot {
                2
            } else {
                checked_mul(payload.max(elements), 4).map_err(fail)?
            },
            observed.compute_work(),
        )
        .and_then(|work| checked_add(work, dense_schema_copy))
        .and_then(|work| checked_add(work, elements))
        .map_err(fail)?;
        admit(|| {
            Ok(budget::resident_cost! {
                compute_work: work,
                comparison_work: observed.comparison_work(),
                output_elements: elements,
                output_bytes: persistent,
                temporary_bytes: checked_add(checked_add(workspace, payload)?, live.0)?,
                cloned_bytes: if exact_snapshot { 0 } else { checked_mul(payload, 3)? },
                retained_nodes: checked_add(checked_mul(source_footprint.node_count, 3)?, live.1)?,
                ..budget::KernelCostEstimate::default()
            })
        })
        .map_err(fail)?;

        if target.region.kind == ResidentValueKind::Snapshot {
            let arena = if target.storage == ResidentStorageClass::Constant {
                &self.activation
            } else {
                &self.workspace.scratch
            };
            let scope = arena
                .prepare_payload_write(target.region)
                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            if let Some(scope) = &scope {
                scope
                    .admit_snapshot_materialization(persistent, workspace)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                scope.start();
            }
            let result = budget::with_payload_admission(
                scope.as_ref().map(|scope| scope.admission()),
                || {
                    if exact_snapshot {
                        let ResidentValueRef::Snapshot([Some(value)]) = source else {
                            unreachable!("exact immutable snapshot checked")
                        };
                        return Ok(value.clone());
                    }
                    let current = crate::resident::numeric::resident_value_to_snapshot(
                        &self.plan.schemas,
                        source_layout.schema_id,
                        source_shape,
                        source_layout.shape,
                        source,
                    )?;
                    current
                        .rebind_with_context(
                            target_slot.schema,
                            &shape,
                            &mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(
                                &self.plan.schemas,
                            ),
                        )
                        .map_err(|_| ResidentKernelError::InvalidShape)
                },
            );
            let arena = if target.storage == ResidentStorageClass::Constant {
                &mut self.activation
            } else {
                &mut self.workspace.scratch
            };
            let next = match result {
                Ok(next) => next,
                Err(error) => {
                    arena
                        .abort_payload_write(target.region, scope)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    return Err(fail(error));
                }
            };
            if let Some(prepared) = &scope {
                if let Err(error) = prepared.admit_value(&next) {
                    arena
                        .abort_payload_write(target.region, scope)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    return Err(ResidentExecutionError::MemoryRuntime { error });
                }
            }
            let ResidentValueMut::Snapshot([value]) = arena.write(target.region) else {
                return Err(fail(ResidentKernelError::InvalidOutput));
            };
            *value = Some(next);
            arena
                .finish_payload_write(target.region, scope)
                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            return Ok(());
        }

        if elements != target.region.len as u64 {
            return Err(fail(ResidentKernelError::InvalidShape));
        }
        // A canonical source is already immutable. Clone its small owning
        // handle only after admission, then use the ordinary typed writer.
        if let ResidentValueRef::Snapshot([Some(value)]) = source {
            let value = value.clone();
            let arena = if target.storage == ResidentStorageClass::Constant {
                &mut self.activation
            } else {
                &mut self.workspace.scratch
            };
            return crate::resident_value_adapter::write_value(arena, target.region, &value)
                .map_err(|error| match error {
                    super::super::ResidentActivationError::MemoryRuntime { error } => {
                        ResidentExecutionError::MemoryRuntime { error }
                    }
                    _ => fail(ResidentKernelError::InvalidOutput),
                });
        }
        if source.kind() != target.region.kind || source.len() != target.region.len {
            return Err(fail(ResidentKernelError::InvalidShape));
        }
        let copied = if target.storage == ResidentStorageClass::Constant {
            match source_location {
                ResidentReadLocation::Constant(region) => {
                    self.activation.copy_region_within(target.region, region)
                }
                ResidentReadLocation::Scratch(region) => {
                    self.activation
                        .copy_region_from(target.region, &self.workspace.scratch, region)
                }
                _ => return Err(fail(ResidentKernelError::InvalidInput)),
            }
        } else {
            match source_location {
                ResidentReadLocation::Constant(region) => {
                    self.workspace
                        .scratch
                        .copy_region_from(target.region, &self.activation, region)
                }
                ResidentReadLocation::Input(region) => self.workspace.scratch.copy_region_from(
                    target.region,
                    &self.workspace.input,
                    region,
                ),
                ResidentReadLocation::LexicalInput(region) => {
                    if let Some(value) =
                        lexical_capture(&self.workspace.continuation_capture_frames, region)
                    {
                        copy_input(&mut self.workspace.scratch, target.region, value.as_ref())
                            .map_err(|error| error.at(fail(ResidentKernelError::InvalidInput)))?;
                        return Ok(());
                    }
                    self.workspace.scratch.copy_region_from(
                        target.region,
                        &self.workspace.input,
                        region,
                    )
                }
                ResidentReadLocation::Scratch(region) => self
                    .workspace
                    .scratch
                    .copy_region_within(target.region, region),
                ResidentReadLocation::State { slot, region } => {
                    let buffer = self.state.select_buffer(slot, epoch);
                    self.workspace.scratch.copy_region_from(
                        target.region,
                        &self.state.buffers[buffer],
                        region,
                    )
                }
            }
        };
        copied.map_err(|error| ResidentExecutionError::MemoryRuntime { error })
    }
}
