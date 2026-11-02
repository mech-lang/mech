//! Shape-preserving scalar lanes for bracketed counts. Scheduling is serial,
//! in canonical element order; no lane publishes a partial enclosing result.
use super::*;
use mech_core::snapshot::canonical_sequence_element_retained_footprint;
use mech_core::snapshot::{SequenceView, SnapshotValidationContext, ValueDataDraft, ValueDraft};
use mech_core::{ManagedMemoryReservation, SchemaBody, SchemaId, ShapeInstance, ValueData};

fn exact_count(values: SequenceView<'_>, lane: usize) -> Result<u64, ResidentKernelError> {
    let invalid = || ResidentKernelError::InvalidInput;
    macro_rules! integer {
        ($values:expr) => {
            $values
                .get(lane)
                .copied()
                .and_then(|value| u64::try_from(value).ok())
                .ok_or_else(invalid)
        };
    }
    fn float(value: f64) -> Result<u64, ResidentKernelError> {
        // Comparing against 2^64 avoids accepting the rounded-out-of-range
        // endpoint. Integer inputs never pass through floating point.
        if value.is_finite()
            && value >= 0.0
            && value.fract() == 0.0
            && value < 18_446_744_073_709_551_616.0
        {
            Ok(value as u64)
        } else {
            Err(ResidentKernelError::InvalidInput)
        }
    }
    match values {
        SequenceView::U8(values) => integer!(values),
        SequenceView::U16(values) => integer!(values),
        SequenceView::U32(values) => integer!(values),
        SequenceView::U64(values) | SequenceView::Index(values) => integer!(values),
        SequenceView::U128(values) => integer!(values),
        SequenceView::I8(values) => integer!(values),
        SequenceView::I16(values) => integer!(values),
        SequenceView::I32(values) => integer!(values),
        SequenceView::I64(values) => integer!(values),
        SequenceView::I128(values) => integer!(values),
        SequenceView::F32(values) => float(values.get(lane).ok_or_else(invalid)?.to_f32() as f64),
        SequenceView::F64(values) => float(values.get(lane).ok_or_else(invalid)?.to_f64()),
        _ => Err(invalid()),
    }
}

fn extents(
    value: &Value,
    schemas: &mech_core::SchemaTable,
) -> Result<Option<[u64; 2]>, ResidentKernelError> {
    let schema = schemas
        .get(value.schema())
        .ok_or(ResidentKernelError::InvalidInput)?;
    match schema.body() {
        SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2 => Ok(Some([
            value
                .shape()
                .resolve_dimension(&dimensions[0])
                .map_err(|_| ResidentKernelError::InvalidShape)?,
            value
                .shape()
                .resolve_dimension(&dimensions[1])
                .map_err(|_| ResidentKernelError::InvalidShape)?,
        ])),
        SchemaBody::Matrix { .. } => Err(ResidentKernelError::InvalidShape),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::snapshot::F64Bits;

    #[test]
    fn iteration_counts_keep_exact_integer_endpoints_and_reject_rounded_float_endpoint() {
        assert_eq!(exact_count(SequenceView::U64(&[u64::MAX]), 0), Ok(u64::MAX));
        assert_eq!(
            exact_count(SequenceView::U128(&[u64::MAX as u128]), 0),
            Ok(u64::MAX)
        );
        assert!(exact_count(SequenceView::U128(&[u64::MAX as u128 + 1]), 0).is_err());
        assert!(exact_count(SequenceView::I64(&[-1]), 0).is_err());
        for value in [u64::MAX as f64, 1.5, -1.0, f64::INFINITY, f64::NAN] {
            assert!(exact_count(SequenceView::F64(&[F64Bits::from_f64(value)]), 0).is_err());
        }
        assert_eq!(
            exact_count(SequenceView::F64(&[F64Bits::from_f64(100.0)]), 0),
            Ok(100)
        );
    }
}

impl ReactiveInstance {
    /// Admission precedes dense materialization. The common adapter attaches
    /// retained managed claims; already canonical inputs share immutable roots.
    fn iteration_snapshot(
        &self,
        node: NodeId,
        read: ResidentReadLocation,
        layout: &ResidentPortLayout,
        epoch: InstanceEpoch,
        live: (u64, u64),
    ) -> Result<Value, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let value = self
            .read_location(read, epoch)
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
        if let ResidentValueRef::Snapshot([Some(value)]) = value {
            if value.schema() != layout.schema_id || value.schema_key() != layout.schema_key {
                return Err(fail(ResidentKernelError::InvalidInput));
            }
            admit(|| Ok(budget::resident_cost! { compute_work: 1, ..budget::KernelCostEstimate::default() })).map_err(fail)?;
            return Ok(value.clone());
        }
        admit(|| Ok(budget::resident_cost! { compute_work: value.len() as u64, ..budget::KernelCostEstimate::default() })).map_err(fail)?;
        let cost = crate::resident::numeric::resident_value_snapshot_materialization_cost(
            &self.plan.schemas,
            layout.schema_id,
            &layout.shape_instance,
            value,
        )
        .map_err(fail)?;
        admit(|| Ok(budget::resident_cost! {
            compute_work: cost.compute_work,
            comparison_work: cost.comparison_work,
            temporary_bytes: checked_add(checked_add(cost.temporary_bytes, cost.persistent_bytes)?, live.0)?,
            cloned_bytes: cost.cloned_bytes,
            retained_nodes: checked_add(cost.retained_nodes, live.1)?,
            ..budget::KernelCostEstimate::default()
        })).map_err(fail)?;
        let capacity = checked_add(cost.temporary_bytes, cost.persistent_bytes).map_err(fail)?;
        let mut reservation = self
            .memory_budget()
            .map(|budget| budget.reserve_capacity(capacity))
            .transpose()
            .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
        let value = crate::resident::numeric::resident_value_to_shared_snapshot(
            &self.plan.schemas,
            layout.schema_id,
            &layout.shape_instance,
            layout.shape,
            value,
        )
        .map_err(fail)?;
        if let Some(reservation) = &mut reservation {
            return value
                .into_memory_budget(reservation)
                .map_err(|error| ResidentExecutionError::MemoryRuntime { error });
        }
        Ok(value)
    }

    fn iteration_store(
        &mut self,
        node: NodeId,
        target: super::super::super::ResidentWriteLocation,
        value: &Value,
    ) -> Result<(), ResidentExecutionError> {
        let slot = &self.plan.slots[target.slot.get() as usize];
        let schema = self
            .plan
            .schemas
            .get(slot.schema)
            .ok_or(ResidentExecutionError::Kernel {
                node,
                error: ResidentKernelError::InvalidOutput,
            })?;
        if value.schema() != slot.schema
            || value.schema_key() != slot.schema_key
            || (slot.activation_fixed_shape && value.shape() != &slot.shape)
            || !mech_core::shape_change_allowed(schema, &slot.shape, value.shape())
        {
            return Err(ResidentExecutionError::Kernel {
                node,
                error: ResidentKernelError::InvalidShape,
            });
        }
        let arena = if target.storage == ResidentStorageClass::Constant {
            &mut self.activation
        } else {
            &mut self.workspace.scratch
        };
        crate::resident_value_adapter::write_value(arena, target.region, value).map_err(|error| {
            match error {
                ResidentActivationError::MemoryRuntime { error } => {
                    ResidentExecutionError::MemoryRuntime { error }
                }
                _ => ResidentExecutionError::Kernel {
                    node,
                    error: ResidentKernelError::InvalidOutput,
                },
            }
        })
    }

    pub(super) fn execute_element_iterations(
        &mut self,
        control: &ActivatedIterationNode,
        count_read: ResidentReadLocation,
        count_layout: &ResidentPortLayout,
        epochs: (InstanceEpoch, InstanceEpoch),
        probe: &mut ResidentStructuralProbe,
        mut live: (u64, u64),
    ) -> Result<(), ResidentExecutionError> {
        let node = control.artifact_node;
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let seed =
            self.iteration_snapshot(node, control.seed, &control.seed_layout, epochs.1, live)?;
        let counts = self.iteration_snapshot(node, count_read, count_layout, epochs.1, live)?;
        admit(|| Ok(budget::resident_cost! { compute_work: 16, comparison_work: 16, ..budget::KernelCostEstimate::default() })).map_err(fail)?;
        let seed_extents = extents(&seed, &self.plan.schemas).map_err(fail)?;
        let count_extents = extents(&counts, &self.plan.schemas)
            .map_err(fail)?
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let ValueData::Matrix(count_matrix) = counts.data() else {
            return Err(fail(ResidentKernelError::InvalidInput));
        };
        let counts_data = count_matrix.elements();
        if count_extents != [1, 1] && Some(count_extents) != seed_extents {
            return Err(fail(ResidentKernelError::InvalidShape));
        }
        let lanes = match seed_extents {
            Some([rows, columns]) => checked_mul(rows, columns).map_err(fail)?,
            None => 1,
        };
        if counts_data.len() as u64
            != checked_mul(count_extents[0], count_extents[1]).map_err(fail)?
        {
            return Err(fail(ResidentKernelError::InvalidShape));
        }
        admit(|| Ok(budget::resident_cost! { compute_work: counts_data.len() as u64, ..budget::KernelCostEstimate::default() })).map_err(fail)?;
        // Validate all counts before invoking the callback in any lane.
        for lane in 0..counts_data.len() {
            exact_count(counts_data, lane).map_err(fail)?;
        }
        let mut meter = budget::ResidentBudgetMeter::default();
        for value in [&seed, &counts] {
            let footprint = budget::measure_borrowed_canonical_value_footprint(
                &mut meter,
                value,
                &self.plan.schemas,
            )
            .map_err(fail)?;
            live.0 = checked_add(live.0, footprint.retained_bytes).map_err(fail)?;
            live.1 = checked_add(live.1, footprint.node_count).map_err(fail)?;
        }
        admit(|| Ok(meter.estimate())).map_err(fail)?;
        let capacity =
            checked_mul(lanes, core::mem::size_of::<ValueDataDraft>() as u64).map_err(fail)?;
        admit(|| Ok(budget::resident_cost! { compute_work: lanes, output_elements: lanes, temporary_bytes: checked_add(capacity, live.0)?, retained_nodes: live.1, ..budget::KernelCostEstimate::default() })).map_err(fail)?;
        let mut reservation = self
            .memory_budget()
            .map(|budget| budget.reserve_capacity(capacity))
            .transpose()
            .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(
                usize::try_from(lanes).map_err(|_| fail(ResidentKernelError::InvalidShape))?,
            )
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        let carrier_slot = &self.plan.slots[control.carrier.slot.get() as usize];
        let carrier_layout = super::super::super::slot_port_layout(carrier_slot);
        let carrier_schema = carrier_slot.schema;
        let scalar_shape = self
            .plan
            .schemas
            .get(carrier_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        let mut retained_draft = capacity;
        let mut retained_nodes = 0;
        for lane in
            0..usize::try_from(lanes).map_err(|_| fail(ResidentKernelError::InvalidShape))?
        {
            budget::charge_planning_step()?;
            let count = exact_count(counts_data, if count_extents == [1, 1] { 0 } else { lane })
                .map_err(fail)?;
            let (footprint, data) = if let ValueData::Matrix(matrix) = seed.data() {
                let body = self.plan.schemas.get(carrier_schema).unwrap().body();
                let footprint =
                    canonical_sequence_element_retained_footprint(body, matrix.elements(), lane)
                        .map_err(|_| fail(ResidentKernelError::InvalidInput))?;
                self.admit_iteration_draft(
                    node,
                    footprint,
                    retained_draft,
                    live,
                    &mut reservation,
                )?;
                (
                    footprint,
                    crate::resident::numeric::sequence_data_draft_at(body, matrix.elements(), lane)
                        .map_err(fail)?,
                )
            } else {
                let mut meter = budget::ResidentBudgetMeter::default();
                let footprint = budget::measure_borrowed_canonical_value_footprint(
                    &mut meter,
                    &seed,
                    &self.plan.schemas,
                )
                .map_err(fail)?;
                admit(|| Ok(meter.estimate())).map_err(fail)?;
                self.admit_iteration_draft(
                    node,
                    footprint,
                    retained_draft,
                    live,
                    &mut reservation,
                )?;
                (
                    footprint,
                    seed.canonical_data_draft()
                        .map_err(|_| fail(ResidentKernelError::InvalidInput))?,
                )
            };
            let value = self.finalize_iteration_draft(node, carrier_schema, &scalar_shape, data)?;
            self.iteration_store(node, control.carrier, &value)?;
            drop(value);
            let body_live = (
                checked_add(live.0, retained_draft)
                    .and_then(|n| checked_add(n, footprint.retained_bytes))
                    .map_err(fail)?,
                checked_add(live.1, retained_nodes).map_err(fail)?,
            );
            self.execute_iteration_body(control, count, epochs, probe, body_live)?;
            let result = self.iteration_snapshot(
                node,
                ResidentReadLocation::Scratch(control.carrier.region),
                &carrier_layout,
                epochs.1,
                body_live,
            )?;
            let mut meter = budget::ResidentBudgetMeter::default();
            let footprint = budget::measure_borrowed_canonical_value_footprint(
                &mut meter,
                &result,
                &self.plan.schemas,
            )
            .map_err(fail)?;
            admit(|| Ok(meter.estimate())).map_err(fail)?;
            self.admit_iteration_draft(node, footprint, retained_draft, live, &mut reservation)?;
            values.push(
                result
                    .canonical_data_draft()
                    .map_err(|_| fail(ResidentKernelError::InvalidOutput))?,
            );
            retained_draft = checked_add(retained_draft, footprint.retained_bytes).map_err(fail)?;
            retained_nodes = checked_add(retained_nodes, footprint.node_count).map_err(fail)?;
            self.workspace
                .scratch
                .discard_payload_write(control.carrier.region);
        }
        // Reserve the complete draft/finalization overlap before constructing
        // an immutable result. Primitive/String lanes have no hidden subtrees.
        self.admit_iteration_draft(
            node,
            ValueFootprint {
                encoded_bytes: retained_draft,
                retained_bytes: retained_draft,
                node_count: checked_add(retained_nodes, 3).map_err(fail)?,
            },
            retained_draft,
            live,
            &mut reservation,
        )?;
        let output_schema = self.plan.slots[control.write.slot.get() as usize].schema;
        let shape = match seed_extents {
            Some(extents) => mech_core::shape_for_resolved_extents(
                self.plan.schemas.get(output_schema).unwrap(),
                &extents,
            )
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?,
            None => scalar_shape,
        };
        let data = if seed_extents.is_some() {
            ValueDataDraft::Matrix(values.into_boxed_slice())
        } else {
            values
                .pop()
                .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?
        };
        let result = self.finalize_iteration_draft(node, output_schema, &shape, data)?;
        self.iteration_store(node, control.write, &result)
    }

    fn admit_iteration_draft(
        &self,
        node: NodeId,
        footprint: ValueFootprint,
        retained: u64,
        live: (u64, u64),
        reservation: &mut Option<ManagedMemoryReservation>,
    ) -> Result<(), ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let footprint = CurrentMemoryFootprint {
            payload_bytes: footprint.retained_bytes,
            encoded_bytes: footprint.encoded_bytes,
            retained_nodes: footprint.node_count,
            logical_elements: 1,
            shape_parameter_count: 2,
            ..CurrentMemoryFootprint::default()
        };
        let draft = mech_core::canonical_snapshot_draft_bytes(footprint)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        let finalization = mech_core::canonical_snapshot_finalization_bytes(footprint)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        let bytes =
            checked_add(retained, checked_add(draft, finalization).map_err(fail)?).map_err(fail)?;
        admit(|| Ok(budget::resident_cost! {
            compute_work: checked_mul(checked_add(footprint.encoded_bytes, footprint.retained_nodes)?, 4)?,
            comparison_work: footprint.encoded_bytes,
            cloned_bytes: checked_mul(footprint.payload_bytes, 3)?,
            temporary_bytes: checked_add(bytes, live.0)?,
            output_bytes: finalization,
            retained_nodes: checked_add(footprint.retained_nodes, live.1)?,
            ..budget::KernelCostEstimate::default()
        })).map_err(fail)?;
        if let Some(reservation) = reservation {
            reservation
                .resize_capacity(bytes)
                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
        }
        Ok(())
    }

    fn finalize_iteration_draft(
        &self,
        node: NodeId,
        schema: SchemaId,
        shape: &ShapeInstance,
        data: ValueDataDraft,
    ) -> Result<Value, ResidentExecutionError> {
        ValueDraft {
            schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &self.plan.schemas,
        ))
        .map_err(|_| ResidentExecutionError::Kernel {
            node,
            error: ResidentKernelError::InvalidShape,
        })
    }
}
