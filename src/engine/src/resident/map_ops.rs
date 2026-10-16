//! Admitted ordinary String-count maps and aligned row-vector projections.
use std::sync::Arc;

use super::budget::{self, PreparedKernel, ResidentBudgetMeter};
use crate::intrinsics::map_ops::{self as primitives, Operation};
use mech_core::snapshot::{
    MapEntryDraft, SequenceView, SnapshotCanonicalizationBudget, SnapshotValidationContext,
};
use mech_core::{
    BoundResidentKernel, FunctionCatalogBuilder, MResult, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentShape, ResidentSnapshotOutput, ResidentValueKind,
    ResidentValueMut, ResidentValueRef, ResolvedOperationContract, SchemaBody, SchemaId, SchemaKey,
    Value, ValueData, ValueDataDraft, ValueDraft,
};

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(
        ["map"],
        "histogram",
        Operation::Histogram.memory(),
        |request| bind(request, Operation::Histogram),
    )?;
    builder.insert_resident_factory(["map"], "keys", Operation::Keys.memory(), |request| {
        bind(request, Operation::Keys)
    })?;
    builder.insert_resident_factory(["map"], "values", Operation::Values.memory(), |request| {
        bind(request, Operation::Values)
    })?;
    Ok(())
}

#[derive(Clone, Debug)]
struct Plan {
    operation: Operation,
    input_schema: SchemaId,
    input_key: SchemaKey,
    input_body: SchemaBody,
    dense_input_count: Option<usize>,
}

fn bind(
    request: &ResidentKernelBindRequest<'_>,
    operation: Operation,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(actual) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let expected = operation.contract();
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let [actual_input] = actual.inputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [actual_output] = actual.outputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let expected_inputs = expected
        .inputs
        .resolve(1)
        .map_err(|_| ResidentKernelBindError::UnsupportedContract)?;
    let policy = &expected.outputs[0];
    if actual.interaction != expected.interaction
        || actual_input.schema != input.schema_id
        || actual_input.access != expected_inputs[0].access
        || actual_input.delivery != expected_inputs[0].delivery
        || actual_output.schema != request.output.schema_id
        || actual_output.access != policy.access
        || actual_output.delivery != policy.delivery
        || actual_output.construction != policy.construction
        || actual_output.alias != policy.alias
        || actual_output.change_detection != policy.change_detection
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let source = request
        .schemas
        .get(input.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let target = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let dense_input_count = match operation {
        Operation::Histogram => {
            if !primitives::is_string_matrix(source.body())
                || !primitives::is_count_map(target.body())
            {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            }
            match input.kind {
                ResidentValueKind::String => {
                    let SchemaBody::Matrix { dimensions, .. } = source.body() else {
                        unreachable!()
                    };
                    let rows = input
                        .shape_instance
                        .resolve_dimension(&dimensions[0])
                        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
                    let columns = input
                        .shape_instance
                        .resolve_dimension(&dimensions[1])
                        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
                    if u64::from(input.shape.rows) != rows
                        || u64::from(input.shape.columns) != columns
                    {
                        return Err(ResidentKernelBindError::UnsupportedLayout);
                    }
                    Some(
                        usize::try_from(
                            rows.checked_mul(columns)
                                .ok_or(ResidentKernelBindError::UnsupportedLayout)?,
                        )
                        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?,
                    )
                }
                ResidentValueKind::Snapshot if input.shape == ResidentShape::SCALAR => None,
                _ => return Err(ResidentKernelBindError::UnsupportedLayout),
            }
        }
        Operation::Keys | Operation::Values => {
            if !primitives::is_count_map(source.body())
                || input.kind != ResidentValueKind::Snapshot
                || input.shape != ResidentShape::SCALAR
            {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            }
            let SchemaBody::Matrix {
                element,
                dimensions,
            } = target.body()
            else {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            };
            let expected = if operation == Operation::Keys {
                SchemaBody::String
            } else {
                SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
            };
            if element.as_ref() != &expected
                || dimensions.len() != 2
                || request
                    .output
                    .shape_instance
                    .resolve_dimension(&dimensions[0])
                    .ok()
                    != Some(1)
            {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            }
            None
        }
    };
    let output_supported = request.output.kind == ResidentValueKind::Snapshot
        && request.output.shape == ResidentShape::SCALAR
        || operation == Operation::Keys
            && request.output.kind == ResidentValueKind::String
            && request.output.shape.rows == 1
            && matches!(target.body(), SchemaBody::Matrix { dimensions, .. }
                if request.output.shape_instance.resolve_dimension(&dimensions[1]).ok()
                    == Some(u64::from(request.output.shape.columns)));
    if !output_supported {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(execute, Box::new([]))
        .with_retained_state(Arc::new(Plan {
            operation,
            input_schema: input.schema_id,
            input_key: input.schema_key,
            input_body: source.body().clone(),
            dense_input_count,
        }))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone())
        // Only the maintained canonical Keys path has been audited here.
        // Its entry scan, payload construction and comparison are admitted
        // before work, and finalization retains the existing schema owner.
        .with_work_accounting(
            if operation == Operation::Keys && request.output.kind == ResidentValueKind::Snapshot {
                ResidentKernelWorkAccounting::RuntimeMetered
            } else {
                ResidentKernelWorkAccounting::Unclassified
            },
        ))
}

#[derive(Clone, Copy)]
enum Strings<'a> {
    Dense(&'a [String]),
    Canonical(&'a [Box<str>]),
}
impl<'a> Strings<'a> {
    fn len(self) -> usize {
        match self {
            Self::Dense(v) => v.len(),
            Self::Canonical(v) => v.len(),
        }
    }
    fn get(self, index: usize) -> &'a str {
        match self {
            Self::Dense(v) => &v[index],
            Self::Canonical(v) => &v[index],
        }
    }
}
fn snapshot<'a>(
    plan: &Plan,
    input: ResidentValueRef<'a>,
) -> Result<&'a Value, ResidentKernelError> {
    let ResidentValueRef::Snapshot([Some(value)]) = input else {
        return Err(ResidentKernelError::InvalidInput);
    };
    if value.schema() != plan.input_schema || value.schema_key() != plan.input_key {
        return Err(ResidentKernelError::InvalidInput);
    }
    Ok(value)
}
fn strings<'a>(
    plan: &Plan,
    input: ResidentValueRef<'a>,
) -> Result<Strings<'a>, ResidentKernelError> {
    if let ResidentValueRef::String(values) = input {
        if Some(values.len()) != plan.dense_input_count {
            return Err(ResidentKernelError::InvalidShape);
        }
        return Ok(Strings::Dense(values));
    }
    let value = snapshot(plan, input)?;
    let ValueData::Matrix(matrix) = value.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let SequenceView::String(values) = matrix.elements() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let SchemaBody::Matrix { dimensions, .. } = &plan.input_body else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let count = dimensions
        .iter()
        .try_fold(1u64, |count, dim| {
            count.checked_mul(value.shape().resolve_dimension(dim).ok()?)
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    if usize::try_from(count).ok() != Some(values.len()) {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(Strings::Canonical(values))
}

fn execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let plan = kernel
        .retained_state::<Plan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let input = inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?;
    let mut meter = ResidentBudgetMeter::default();
    let mut order = Vec::new();
    let mut scratch = Vec::new();
    let source_strings = if plan.operation == Operation::Histogram {
        Some(strings(plan, input)?)
    } else {
        None
    };
    let source_map = if source_strings.is_none() {
        let ValueData::Map(map) = snapshot(plan, input)?.data() else {
            return Err(ResidentKernelError::InvalidInput);
        };
        Some(map)
    } else {
        None
    };
    let (count, payload, work, scratch_bytes) = if let Some(strings) = source_strings {
        let n = strings.len();
        meter.charge_compute_work(budget::checked_u64(n)?)?;
        let bytes = (0..n).try_fold(0usize, |total, index| {
            total
                .checked_add(strings.get(index).len())
                .ok_or(ResidentKernelError::InvalidShape)
        })?;
        let levels = if n <= 1 {
            0
        } else {
            usize::BITS - (n - 1).leading_zeros()
        } as usize;
        let work = bytes
            .checked_add(n)
            .and_then(|value| value.checked_mul(levels.checked_add(6)?))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let scratch_bytes = n
            .checked_mul(core::mem::size_of::<usize>())
            .and_then(|bytes| bytes.checked_mul(2))
            .ok_or(ResidentKernelError::InvalidShape)?;
        PreparedKernel::new((), budget::resident_cost! { compute_work: work, comparison_work: work, temporary_bytes: scratch_bytes, ..budget::KernelCostEstimate::default() }).admit()?.into_plan();
        order
            .try_reserve_exact(n)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        scratch
            .try_reserve_exact(n)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        order.resize(n, 0);
        scratch.resize(n, 0);
        primitives::sort_positions(&mut order, &mut scratch, |index| strings.get(index));
        let mut count = 0usize;
        let mut payload = 0usize;
        let mut start = 0;
        while start < n {
            payload = payload
                .checked_add(strings.get(order[start]).len())
                .ok_or(ResidentKernelError::InvalidShape)?;
            start = primitives::run_end(&order, start, |index| strings.get(index));
            count += 1;
        }
        (count, payload, work, scratch_bytes)
    } else {
        let entries = source_map
            .ok_or(ResidentKernelError::InvalidInput)?
            .entries();
        meter.charge_compute_work(budget::checked_u64(entries.len())?)?;
        let mut payload = 0usize;
        for entry in entries {
            let (ValueData::String(text), ValueData::U64(_)) = (entry.key().data(), entry.value())
            else {
                return Err(ResidentKernelError::InvalidInput);
            };
            if plan.operation == Operation::Keys {
                payload = payload
                    .checked_add(text.len())
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
        }
        (entries.len(), payload, entries.len(), 0)
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let shape_bytes = metadata
        .shape
        .parameter_values()
        .len()
        .checked_mul(32)
        .and_then(|bytes| bytes.checked_add(512))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            compute_work: shape_bytes, temporary_bytes: shape_bytes,
            ..budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let shape = if plan.operation == Operation::Histogram {
        metadata.shape.clone()
    } else {
        mech_core::shape_for_resolved_extents(schema, &[1, budget::checked_u64(count)?])
            .map_err(|_| ResidentKernelError::InvalidShape)?
    };
    let footprint = primitives::prospective(
        plan.operation,
        budget::checked_u64(count)?,
        budget::checked_u64(payload)?,
        shape.parameter_values().len(),
    )
    .map_err(|_| ResidentKernelError::InvalidShape)?;
    let comparison_work = match &output {
        ResidentValueMut::Snapshot([Some(current)]) => {
            let previous = budget::published_canonical_footprint(&mut meter, current, schemas)?;
            budget::projected_language_equality_work(
                schemas,
                current,
                previous,
                metadata.schema,
                shape.parameter_values().len(),
                footprint,
            )?
        }
        ResidentValueMut::Snapshot([None]) => 0,
        ResidentValueMut::String(values)
            if plan.operation == Operation::Keys && values.len() == count =>
        {
            values
                .iter()
                .try_fold(budget::checked_u64(payload)?, |sum, text| {
                    sum.checked_add(budget::checked_u64(text.len())?)
                        .ok_or(ResidentKernelError::InvalidShape)
                })?
        }
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    let finalization_work = budget::checked_u64(payload)?
        .checked_mul(3)
        .and_then(|v| v.checked_add((count as u64).checked_mul(8)?))
        .and_then(|v| v.checked_add(8))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let drafts = count
        .checked_mul(
            core::mem::size_of::<MapEntryDraft>()
                + 2 * core::mem::size_of::<ValueDataDraft>()
                + core::mem::size_of::<String>(),
        )
        .ok_or(ResidentKernelError::InvalidShape)?;
    let temporary = (scratch_bytes as u64)
        .checked_add(shape_bytes as u64)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(drafts as u64)
        .and_then(|v| v.checked_add((payload as u64).checked_mul(2)?))
        .and_then(|v| v.checked_add(footprint.retained_bytes.checked_mul(2)?))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let measured = meter.estimate();
    PreparedKernel::new((), budget::resident_cost! {
        compute_work: (work as u64).checked_add(finalization_work).and_then(|v| v.checked_add(measured.compute_work())).ok_or(ResidentKernelError::InvalidShape)?,
        comparison_work: comparison_work.checked_add(finalization_work).and_then(|v| v.checked_add(work as u64)).and_then(|v| v.checked_add(measured.comparison_work())).ok_or(ResidentKernelError::InvalidShape)?,
        output_elements: count, output_bytes: footprint.retained_bytes, temporary_bytes: temporary, cloned_bytes: payload,
        retained_nodes: footprint.node_count.checked_mul(3).and_then(|v| v.checked_add(measured.retained_nodes())).ok_or(ResidentKernelError::InvalidShape)?,
        ..budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    let data = if let Some(strings) = source_strings {
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        let mut start = 0;
        while start < order.len() {
            let end = primitives::run_end(&order, start, |index| strings.get(index));
            let count =
                primitives::count_run(start, end).map_err(|_| ResidentKernelError::InvalidShape)?;
            entries.push(MapEntryDraft {
                items: vec![
                    ValueDataDraft::String(strings.get(order[start]).to_owned()),
                    ValueDataDraft::U64(count),
                ]
                .into_boxed_slice(),
            });
            start = end;
        }
        ValueDataDraft::Map(entries.into_boxed_slice())
    } else {
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        for entry in source_map.unwrap().entries() {
            values.push(if plan.operation == Operation::Keys {
                let ValueData::String(text) = entry.key().data() else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                ValueDataDraft::String(text.to_string())
            } else {
                let ValueData::U64(count) = entry.value() else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                ValueDataDraft::U64(*count)
            });
        }
        ValueDataDraft::Matrix(values.into_boxed_slice())
    };
    let finalization_budget = SnapshotCanonicalizationBudget::new(finalization_work);
    let context = if plan.operation == Operation::Keys {
        // Reuse the immutable owner retained by the actual bound kernel. A
        // whole-arena clone is not part of the per-key materialization demand.
        SnapshotValidationContext::with_shared_schemas(
            kernel
                .shared_snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidOutput)?,
        )
    } else {
        SnapshotValidationContext::new(schemas)
    }
    .with_canonicalization_budget(&finalization_budget);
    let next = ValueDraft {
        schema: metadata.schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&context)
    .map_err(|_| ResidentKernelError::InvalidOutput)?;
    if next.schema_key() != metadata.schema_key {
        return Err(ResidentKernelError::InvalidOutput);
    }
    match output {
        ResidentValueMut::Snapshot([target]) => {
            let changed = match target.as_ref() {
                Some(current) => !current
                    .snapshot_eq(schemas, &next, schemas)
                    .map_err(|_| ResidentKernelError::InvalidOutput)?,
                None => true,
            };
            *target = Some(next);
            Ok(changed)
        }
        ResidentValueMut::String(target) => {
            let ValueData::Matrix(matrix) = next.data() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            let SequenceView::String(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            // All fallible construction/validation precedes the first update.
            let changed = target
                .iter()
                .zip(values)
                .any(|(old, new)| old.as_str() != new.as_ref());
            let candidate = values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();
            for (target, next) in target.iter_mut().zip(candidate) {
                *target = next;
            }
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        AccessMode, AliasPolicy, BoundCall, CallMemoryPlanningRequest, CardinalitySpec,
        CurrentMemoryFootprint, DeclaredOperationContract, DeliveryMode, DimensionExpr,
        ExternalInteraction, IntegerWidth, MemoryFootprintWitness, MemoryLifetime, MemoryPlanPoint,
        NodeId, OutputConstruction, PortDirection, RegionAccessPlan, ResidentOperationKey,
        ResidentPortLayout, ResolvedInputPort, ResolvedOperationDescriptor, ResolvedOutputPort,
        ResolvedValueDescriptor, SchemaDraft, SchemaTableBuilder, ShapeRule, TargetMemoryProfile,
        plan_call_memory,
    };

    struct Inputs<'a>(&'a [ResidentValueRef<'a>]);
    impl ResidentKernelInputs for Inputs<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }
        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0.get(index).copied()
        }
    }

    #[test]
    fn canonical_keys_share_large_schema_owner_and_deny_payload_before_publication() {
        let mut builder = SchemaTableBuilder::new();
        let mut add = |body| {
            builder
                .insert(
                    SchemaDraft {
                        dimension_parameters: Box::new([]),
                        body,
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap()
        };
        let map = add(SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        });
        let keys = add(SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(2)]),
        });
        for columns in 0..512 {
            add(SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: Box::new([
                    DimensionExpr::Constant(1),
                    DimensionExpr::Constant(columns),
                ]),
            });
        }
        let build = builder.finish().unwrap();
        let map = build.resolve(map).unwrap();
        let keys = build.resolve(keys).unwrap();
        let (schemas, _) = build.into_parts();
        let arena_bytes = schemas.clone_allocation_bound_bytes().unwrap();
        assert!(arena_bytes > 4096);
        let layout = |id| {
            let schema = schemas.get(id).unwrap();
            ResidentPortLayout {
                schema_id: id,
                schema_key: schema.key(),
                kind: ResidentValueKind::Snapshot,
                shape: ResidentShape::SCALAR,
                shape_instance: schema.instantiate_shape(Box::new([])).unwrap(),
                activation_fixed_shape: true,
                resolved_selector: None,
            }
        };
        let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: Box::new([ResolvedInputPort {
                schema: map,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]),
            outputs: Box::new([ResolvedOutputPort {
                schema: keys,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: mech_core::ChangeDetectionPolicy::KernelReported,
            }]),
            interaction: ExternalInteraction::Pure,
        });
        // Use the actual maintained projection contract, not an invented policy.
        let policy = Operation::Keys.contract();
        let ResolvedOperationContract::Declared(mut declared) = contract else {
            unreachable!()
        };
        declared.outputs[0].construction = policy.outputs[0].construction.clone();
        declared.outputs[0].change_detection = policy.outputs[0].change_detection;
        let contract = ResolvedOperationContract::Declared(declared);
        let request = ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[layout(map)],
            output: layout(keys),
        };
        let kernel = bind(&request, Operation::Keys).unwrap();
        assert_eq!(
            kernel.work_accounting(),
            ResidentKernelWorkAccounting::RuntimeMetered
        );
        let owner = kernel.shared_snapshot_schemas().unwrap().clone();
        assert!(Arc::ptr_eq(
            &owner,
            kernel.clone().shared_snapshot_schemas().unwrap()
        ));
        assert_eq!(
            kernel
                .clone()
                .with_snapshot_schemas(schemas.clone())
                .work_accounting(),
            ResidentKernelWorkAccounting::Unclassified
        );

        let input = |second: &str| {
            Some(
                ValueDraft {
                    schema: map,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::Map(
                        vec![
                            MapEntryDraft {
                                items: vec![
                                    ValueDataDraft::String("a".into()),
                                    ValueDataDraft::U64(1),
                                ]
                                .into_boxed_slice(),
                            },
                            MapEntryDraft {
                                items: vec![
                                    ValueDataDraft::String(second.into()),
                                    ValueDataDraft::U64(2),
                                ]
                                .into_boxed_slice(),
                            },
                        ]
                        .into_boxed_slice(),
                    ),
                }
                .finalize(&SnapshotValidationContext::new(&schemas))
                .unwrap(),
            )
        };
        let small = [input("z")];
        let large = [input(&"z".repeat(8192))];
        let recovered = [input("zz")];
        let node = NodeId::new(0);
        let descriptor = |id| {
            let schema = schemas.get(id).unwrap().clone();
            let shape = schema.instantiate_shape(Box::new([])).unwrap();
            ResolvedValueDescriptor::from_schema(schema, shape).unwrap()
        };
        let binding = BoundCall::artifact_operation(
            ResolvedOperationDescriptor::from_name(
                Operation::Keys.name(),
                Operation::Keys.contract().clone(),
            )
            .unwrap(),
            Box::new([descriptor(map)]),
            Box::new([descriptor(keys)]),
            ResidentOperationKey {
                module_path: vec!["map".to_owned()].into_boxed_slice(),
                operation_name: "keys".to_owned(),
            },
        )
        .unwrap();
        let accounting = kernel.work_accounting();
        let kernel = kernel
            .with_bound_call(binding.clone())
            .with_work_accounting(accounting);
        let footprint = |value: Option<&Value>, id| {
            let Some(value) = value else {
                return CurrentMemoryFootprint::default();
            };
            let mut meter = ResidentBudgetMeter::default();
            let measured =
                budget::measure_canonical_value_footprint(&mut meter, value, &schemas).unwrap();
            let logical_elements = match value.data() {
                ValueData::Map(value) => value.entries().len(),
                ValueData::Matrix(value) => value.elements().len(),
                other => panic!("{other:?}"),
            };
            CurrentMemoryFootprint {
                logical_elements: logical_elements as u64,
                payload_bytes: measured.retained_bytes,
                encoded_bytes: measured.encoded_bytes,
                retained_nodes: measured.node_count,
                schema_bytes: schemas.get(id).unwrap().canonical_bytes().len() as u64,
                shape_parameter_count: value.shape().parameter_values().len() as u64,
                ..CurrentMemoryFootprint::default()
            }
        };
        let mut target = TargetMemoryProfile::current_resident_cpu().unwrap();
        target.limits.max_output_bytes = Some(4096);
        target.limits.max_temporary_bytes = Some(4096);
        target.limits.max_cloned_bytes = Some(4096);
        let lifetime = MemoryLifetime::Turn {
            first: MemoryPlanPoint::new(0),
            last: MemoryPlanPoint::new(1),
        };
        let source_storage = crate::memory_planner::resident_storage_descriptor(
            &descriptor(map),
            ResidentValueKind::Snapshot,
            lifetime,
        )
        .unwrap();
        let output_storage = crate::memory_planner::resident_storage_descriptor(
            &descriptor(keys),
            ResidentValueKind::Snapshot,
            lifetime,
        )
        .unwrap();
        let call = plan_call_memory(CallMemoryPlanningRequest {
            bound_call: &binding,
            input_storage: &[source_storage],
            output_storage: &[output_storage],
            input_witnesses: &[MemoryFootprintWitness::Known(footprint(
                small[0].as_ref(),
                map,
            ))],
            output_witnesses: &[MemoryFootprintWitness::Known(
                CurrentMemoryFootprint::default(),
            )],
            published_output_witnesses: &[MemoryFootprintWitness::Known(
                CurrentMemoryFootprint::default(),
            )],
            implementation_memory: Operation::Keys.memory(),
            target: &target,
            regions: &[RegionAccessPlan::WholeValue],
        })
        .unwrap();
        let program = crate::memory_planner::ProgramMemoryPlan {
            call_nodes: Box::new([node]),
            calls: Box::new([call.clone()]),
            allocations: call.allocations.clone(),
            budget_limits: target.limits,
            ..crate::memory_planner::ProgramMemoryPlan::default()
        };
        let invoke = |input: &[Option<Value>; 1], output: &mut [Option<Value>; 1]| {
            let mut facts = crate::memory_planner::TurnMemoryFacts::default();
            facts.resolved_footprints.insert(
                (node, PortDirection::Input, 0),
                footprint(input[0].as_ref(), map),
            );
            let prior = footprint(output[0].as_ref(), keys);
            facts
                .resolved_footprints
                .insert((node, PortDirection::Output, 0), prior);
            facts.published_footprints.insert((node, 0), prior);
            let turn =
                crate::memory_planner::plan_current_resident_turn(&program, node, &facts).unwrap();
            assert!(
                turn.call.is_some(),
                "every owned invocation needs its actual bound call"
            );
            assert_eq!(turn.facts.published_footprints[&(node, 0)], prior);
            if !turn.budget_violations.is_empty() {
                // This is a real finite call-plan denial, not a missing old-output witness.
                assert!(
                    turn.budget_violations
                        .iter()
                        .all(|violation| violation.required > violation.limit)
                );
                return Err(ResidentKernelError::InvalidShape);
            }
            budget::with_resident_turn_plan(turn, || {
                kernel.execute(
                    &Inputs(&[ResidentValueRef::Snapshot(input)]),
                    ResidentValueMut::Snapshot(output),
                )
            })
        };
        let mut output = [None];
        invoke(&small, &mut output).unwrap();
        let previous = output[0].as_ref().unwrap().clone();
        assert!(Arc::ptr_eq(&owner, &previous.schemas().unwrap()));
        assert_eq!(
            invoke(&large, &mut output),
            Err(ResidentKernelError::InvalidShape),
            "oversized valid keys must fail a finite admission, never InvalidOutput"
        );
        assert!(
            output[0].as_ref().unwrap().shares_frozen_storage(&previous),
            "denial retains the exact prior publication across fresh invocation scopes"
        );
        invoke(&recovered, &mut output).unwrap();
        let value = output[0].as_ref().unwrap();
        assert!(Arc::ptr_eq(&owner, &value.schemas().unwrap()));
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("{value:?}")
        };
        let SequenceView::String(values) = matrix.elements() else {
            panic!("{matrix:?}")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.as_ref())
                .collect::<Vec<_>>(),
            ["a", "zz"]
        );
    }
}
