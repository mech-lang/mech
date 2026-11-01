//! Shared dense or canonical scalar Bool publication. Predicate semantics
//! remain with each maintained executor; construction is admitted before allocation.
use super::*;

pub(in crate::resident) fn footprint(
    kernel: &BoundResidentKernel,
) -> Result<PublishedOutputFootprint, ResidentKernelError> {
    if kernel.snapshot_output().is_none() {
        return Ok(PublishedOutputFootprint {
            elements: 1,
            retained_bytes: 1,
            retained_nodes: 1,
        });
    }
    let canonical = crate::resident::budget::projected_canonical_value_footprint(
        mech_core::snapshot::ValueFootprint {
            encoded_bytes: 1,
            retained_bytes: checked_u64(core::mem::size_of::<ValueData>())?,
            node_count: 1,
        },
        0,
    )?;
    Ok(PublishedOutputFootprint {
        elements: 1,
        retained_bytes: canonical.retained_bytes,
        retained_nodes: canonical.node_count,
    })
}

pub(in crate::resident) fn bind(
    kernel: BoundResidentKernel,
    request: &ResidentKernelBindRequest<'_>,
) -> BoundResidentKernel {
    if request.output.kind != ResidentValueKind::Snapshot {
        return kernel;
    }
    kernel
        .with_snapshot_schemas(request.schemas.clone())
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
}

pub(in crate::resident) fn write(
    kernel: &BoundResidentKernel,
    output: ResidentValueMut<'_>,
    next: bool,
) -> Result<bool, ResidentKernelError> {
    match output {
        ResidentValueMut::Bool([target]) => {
            let next = u8::from(next);
            let changed = *target != next;
            *target = next;
            Ok(changed)
        }
        ResidentValueMut::Snapshot([target]) => {
            let schemas = kernel
                .shared_snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidOutput)?;
            let metadata = kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?;
            let mut meter = ResidentBudgetMeter::default();
            let previous = target
                .as_ref()
                .map(|value| {
                    crate::resident::budget::measure_canonical_value_footprint(
                        &mut meter, value, schemas,
                    )
                })
                .transpose()?;
            let previous_bool = match target.as_ref().map(Value::data) {
                Some(ValueData::Bool(value)) => Some(*value),
                None => None,
                _ => return Err(ResidentKernelError::InvalidOutput),
            };
            // Constant scalar construction, wrapper/claim and finalization
            // storage; shared schemas do not clone the application arena.
            let allocation = checked_cost_sum(&[
                Value::canonical_owner_allocation_bytes(),
                Value::memory_budget_claim_metadata_bytes(),
                checked_u64(core::mem::size_of::<ValueData>())?,
                256,
            ])?;
            let measured = meter.estimate();
            let cost = crate::resident::budget::resident_cost! {
                comparison_work: checked_cost_sum(&[measured.comparison_work(), 16])?,
                compute_work: checked_cost_sum(&[measured.compute_work(), checked_cost_product(&[allocation, 8])?, 128])?,
                output_elements: 1,
                output_bytes: allocation,
                temporary_bytes: checked_cost_sum(&[previous.map_or(0, |v| v.retained_bytes), checked_cost_product(&[allocation, 2])?])?,
                retained_nodes: checked_cost_sum(&[previous.map_or(0, |v| v.node_count), 8])?,
                ..KernelCostEstimate::default()
            };
            PreparedKernel::new((), cost).admit()?.into_plan();
            let value = ValueDraft {
                schema: metadata.schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Bool(next),
            }
            .finalize(&SnapshotValidationContext::with_shared_schemas(schemas))
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
            *target = Some(value);
            Ok(previous_bool != Some(next))
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel() -> BoundResidentKernel {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let boolean = builder
            .insert(
                mech_core::SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Bool,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let id = build.resolve(boolean).unwrap();
        let (schemas, _) = build.into_parts();
        let schema = schemas.get(id).unwrap();
        let metadata = ResidentSnapshotOutput {
            schema: id,
            schema_key: schema.key(),
            shape: schema.instantiate_shape(Box::new([])).unwrap(),
            exact_cardinality: None,
            maximum_cardinality: None,
        };
        BoundResidentKernel::new(|_, _, _| unreachable!(), Box::new([]))
            .with_snapshot_schemas(schemas)
            .with_snapshot_output(metadata)
    }

    #[test]
    fn canonical_bool_footprint_matches_the_actual_value_not_the_dense_lane() {
        let kernel = kernel();
        let schemas = kernel.shared_snapshot_schemas().unwrap();
        let actual = ValueDraft {
            schema: kernel.snapshot_output().unwrap().schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Bool(false),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(schemas))
        .unwrap();
        let expected = actual.retained_footprint(schemas).unwrap();
        let predicted = footprint(&kernel).unwrap();
        assert_eq!(predicted.elements, 1);
        assert_eq!(predicted.retained_bytes, expected.retained_bytes);
        assert_eq!(predicted.retained_nodes, expected.node_count);
        assert!(predicted.retained_bytes > 1);
    }

    #[test]
    fn canonical_bool_publication_has_exact_change_flags_and_shared_schemas() {
        let kernel = kernel();
        let mut output = [None];
        for (next, changed) in [
            (false, true),
            (false, false),
            (true, true),
            (true, false),
            (false, true),
        ] {
            assert_eq!(
                write(&kernel, ResidentValueMut::Snapshot(&mut output), next),
                Ok(changed)
            );
            let value = output[0].as_ref().unwrap();
            assert!(matches!(value.data(), ValueData::Bool(actual) if *actual == next));
            assert!(Arc::ptr_eq(
                &value.schemas().unwrap(),
                kernel.shared_snapshot_schemas().unwrap()
            ));
        }
    }
}
