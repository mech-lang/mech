//! Borrowed, bounded measurements at the actual Resident execution boundary.
//! No activation-time allocation estimate is evidence of a later payload.

use super::*;
use crate::memory_planner::TurnMemoryFacts;
use mech_core::{CallMemoryPlan, PortDirection, RegionPolicy, ResourceDemand, ValueData};

fn add(left: u64, right: u64) -> Result<u64, MemoryPlanError> {
    left.checked_add(right)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "live Resident footprint",
        })
}

pub(super) fn footprint(
    value: ResidentValueRef<'_>,
    descriptor: &mech_core::ResolvedValueDescriptor,
    schemas: &mech_core::SchemaTable,
    work: &mut ResourceDemand,
) -> Result<CurrentMemoryFootprint, MemoryPlanError> {
    let mut result = CurrentMemoryFootprint {
        logical_elements: value.len() as u64,
        schema_bytes: descriptor.schema().canonical_bytes().len() as u64,
        shape_parameter_count: descriptor.shape().parameter_values().len() as u64,
        ..CurrentMemoryFootprint::default()
    };
    match value {
        ResidentValueRef::String(values) => {
            for value in values {
                result.payload_bytes = add(result.payload_bytes, value.capacity() as u64)?;
                result.encoded_bytes = add(result.encoded_bytes, add(8, value.len() as u64)?)?;
                result.retained_nodes = add(result.retained_nodes, 1)?;
                // Reading a length is O(1), not a scan of the string contents.
                work.work.compute = add(work.work.compute, 1)?;
                check_measurement(result, *work)?;
            }
        }
        ResidentValueRef::Snapshot(values) => {
            for value in values.iter().flatten() {
                let schema = schemas
                    .get(value.schema())
                    .ok_or(MemoryPlanError::DescriptorMismatch)?;
                let shape_bytes = (value.shape().parameter_values().len() as u64)
                    .checked_mul(8)
                    .ok_or(MemoryPlanError::TargetAddressOverflow)?;
                result.payload_bytes = add(
                    result.payload_bytes,
                    add(core::mem::size_of::<Value>() as u64, shape_bytes)?,
                )?;
                result.retained_nodes = add(result.retained_nodes, 1)?;
                mech_core::snapshot::visit_canonical_data_work(
                    schema.body(),
                    value.data(),
                    |chunk| {
                        result.payload_bytes = add(result.payload_bytes, chunk.retained_bytes)?;
                        result.encoded_bytes = add(result.encoded_bytes, chunk.encoded_bytes)?;
                        result.retained_nodes = add(result.retained_nodes, chunk.node_count)?;
                        work.work.compute = add(work.work.compute, chunk.node_count.max(1))?;
                        check_measurement(result, *work)
                    },
                )
                .map_err(|error| match error {
                    mech_core::snapshot::CanonicalDataWorkError::Visitor(error) => error,
                    _ => MemoryPlanError::DescriptorMismatch,
                })?;
            }
            if let [Some(value)] = values {
                result.logical_elements = match value.data() {
                    ValueData::Set(set) => set.elements().len() as u64,
                    ValueData::Map(map) => map.entries().len() as u64,
                    ValueData::Table(table) => {
                        table.column(0).map_or(0, |column| column.len()) as u64
                    }
                    ValueData::Matrix(matrix) => matrix.elements().len() as u64,
                    _ => result.logical_elements,
                };
            }
        }
        ResidentValueRef::Bool(values) => result.encoded_bytes = values.len() as u64,
        ResidentValueRef::Index(values) => {
            result.encoded_bytes = (values.len() as u64)
                .checked_mul(8)
                .ok_or(MemoryPlanError::TargetAddressOverflow)?
        }
        ResidentValueRef::F64(values) => {
            result.encoded_bytes = (values.len() as u64)
                .checked_mul(8)
                .ok_or(MemoryPlanError::TargetAddressOverflow)?
        }
    }
    check_measurement(result, *work)?;
    Ok(result)
}

fn check_measurement(
    f: CurrentMemoryFootprint,
    work: ResourceDemand,
) -> Result<(), MemoryPlanError> {
    let target = TargetMemoryProfile::current_resident_cpu()?;
    let demand = ResourceDemand {
        output_elements: f.logical_elements,
        retained_nodes: f.retained_nodes,
        ..work
    };
    if let Some(violation) = mech_core::evaluate_memory_budget(
        mech_core::MemoryObjectOwner::DirectCallPort {
            call: 0,
            direction: PortDirection::Input,
            port: 0,
        },
        demand,
        f.payload_bytes,
        0,
        target.limits,
    )
    .first()
    {
        return Err(MemoryPlanError::TargetLimitExceeded {
            violation: violation.clone(),
        });
    }
    Ok(())
}

/// All inputs are in semantic port order, including the RMW base that the
/// optimized execution tape omits. The old output is measured independently.
pub(super) fn facts(
    call: &CallMemoryPlan,
    node: NodeId,
    inputs: &[ResidentValueRef<'_>],
    published: ResidentValueRef<'_>,
    schemas: &mech_core::SchemaTable,
) -> Result<TurnMemoryFacts, MemoryPlanError> {
    if inputs.len() != call.inputs.len() || call.outputs.len() != 1 {
        return Err(MemoryPlanError::DescriptorArityMismatch);
    }
    let mut facts = TurnMemoryFacts::default();
    for (ordinal, (value, port)) in inputs.iter().zip(&call.inputs).enumerate() {
        let f = footprint(
            *value,
            &port.descriptor,
            schemas,
            &mut facts.additional_demand,
        )?;
        facts
            .resolved_footprints
            .insert((node, PortDirection::Input, ordinal as u16), f);
    }
    let current = footprint(
        published,
        &call.outputs[0].descriptor,
        schemas,
        &mut facts.additional_demand,
    )?;
    facts.published_footprints.insert((node, 0), current);
    // This seeds the scope, not a candidate materialization permit. The
    // existing bounded kernel preflight supplies/refines the candidate before
    // its first clone, draft, finalization, or publication.
    facts
        .resolved_footprints
        .insert((node, PortDirection::Output, 0), current);
    for (ordinal, output) in call.outputs.iter().enumerate() {
        if let RegionAccessPlan::Deferred(policy) = output.region {
            let region = selected_region(policy, inputs, &output.value, &mut facts)?;
            facts
                .resolved_regions
                .insert((node, ordinal as u16), region);
        }
    }
    Ok(facts)
}

fn selected_count(
    value: ResidentValueRef<'_>,
    work: &mut ResourceDemand,
) -> Result<u64, MemoryPlanError> {
    match value {
        ResidentValueRef::Bool(values) => {
            work.work.compute = add(work.work.compute, 2 * values.len() as u64)?;
            check_measurement(CurrentMemoryFootprint::default(), *work)?;
            if values.iter().any(|v| *v > 1) {
                return Err(MemoryPlanError::DescriptorMismatch);
            }
            Ok(values.iter().filter(|v| **v != 0).count() as u64)
        }
        ResidentValueRef::Snapshot([Some(value)]) => match value.data() {
            ValueData::Matrix(matrix) => match matrix.elements() {
                mech_core::snapshot::SequenceView::Bool(values) => {
                    work.work.compute = add(work.work.compute, values.len() as u64)?;
                    check_measurement(CurrentMemoryFootprint::default(), *work)?;
                    Ok(values.iter().filter(|v| **v).count() as u64)
                }
                values => Ok(values.len() as u64),
            },
            _ => Ok(1),
        },
        value => Ok(value.len() as u64),
    }
}

fn selected_region(
    policy: RegionPolicy,
    inputs: &[ResidentValueRef<'_>],
    output: &mech_core::ValueLayoutPlan,
    facts: &mut TurnMemoryFacts,
) -> Result<RegionAccessPlan, MemoryPlanError> {
    if policy == RegionPolicy::WholeValue {
        return Ok(RegionAccessPlan::WholeValue);
    }
    if policy == RegionPolicy::CollectionEntry {
        return Ok(RegionAccessPlan::CollectionEntry {
            key_bytes: facts
                .resolved_footprints
                .iter()
                .filter(|((_, direction, _), _)| *direction == PortDirection::Input)
                .next_back()
                .map_or(0, |(_, f)| f.encoded_bytes),
        });
    }
    // Indexed RMW's validated port order is base, source, selector(s).
    let selector = inputs
        .last()
        .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
    let mut selected = selected_count(*selector, &mut facts.additional_demand)?;
    match policy {
        RegionPolicy::RectangularRegion if inputs.len() >= 4 => {
            selected = selected
                .checked_mul(selected_count(
                    inputs[inputs.len() - 2],
                    &mut facts.additional_demand,
                )?)
                .ok_or(MemoryPlanError::TargetAddressOverflow)?;
        }
        RegionPolicy::IndexedAxis { axis } => {
            for (ordinal, extent) in output.axes.iter().enumerate() {
                if ordinal != usize::from(axis) {
                    selected = selected
                        .checked_mul(extent.current)
                        .ok_or(MemoryPlanError::TargetAddressOverflow)?;
                }
            }
        }
        _ => {}
    }
    Ok(RegionAccessPlan::Gather {
        selected_elements: selected,
        index_bytes: selected
            .checked_mul(8)
            .ok_or(MemoryPlanError::TargetAddressOverflow)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::snapshot::SnapshotValidationContext;
    use mech_core::{SchemaDraft, SchemaTableBuilder, ValueDataDraft, ValueDraft};

    fn schema(
        body: SchemaBody,
    ) -> (
        mech_core::ResolvedValueDescriptor,
        mech_core::SchemaTable,
        SchemaId,
    ) {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap();
        let shape = schema.instantiate_shape(Box::new([])).unwrap();
        let descriptor =
            mech_core::ResolvedValueDescriptor::from_schema(schema.clone(), shape).unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let built = builder.finish().unwrap();
        let id = built.resolve(handle).unwrap();
        (descriptor, built.into_parts().0, id)
    }

    #[test]
    fn live_string_measurements_follow_growth_shrinkage_and_retained_capacity() {
        let (descriptor, schemas, _) = schema(SchemaBody::String);
        let mut value = [String::new()];
        for text in ["abc".to_owned(), "x".repeat(1024), "z".to_owned()] {
            value[0].clear();
            value[0].push_str(&text);
            let measured = footprint(
                ResidentValueRef::String(&value),
                &descriptor,
                &schemas,
                &mut ResourceDemand::default(),
            )
            .unwrap();
            assert_eq!(measured.logical_elements, 1);
            assert_eq!(measured.payload_bytes, value[0].capacity() as u64);
            assert_eq!(measured.encoded_bytes, text.len() as u64 + 8);
        }
    }

    #[test]
    fn live_recursive_snapshot_measurements_match_canonical_retained_footprints() {
        let (descriptor, schemas, id) = schema(SchemaBody::Set {
            element: Box::new(SchemaBody::Tuple(
                vec![SchemaBody::String, SchemaBody::String].into(),
            )),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        });
        for (count, length) in [(1, 3), (7, 80), (2, 1)] {
            let value = ValueDraft {
                schema: id,
                shape_values: Box::new([]),
                data: ValueDataDraft::Set(
                    (0..count)
                        .map(|i| {
                            ValueDataDraft::Tuple(
                                vec![
                                    ValueDataDraft::String(format!("{i}")),
                                    ValueDataDraft::String("x".repeat(length)),
                                ]
                                .into(),
                            )
                        })
                        .collect::<Vec<_>>()
                        .into(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap();
            let expected = value.retained_footprint(&schemas).unwrap();
            let values = [Some(value)];
            let measured = footprint(
                ResidentValueRef::Snapshot(&values),
                &descriptor,
                &schemas,
                &mut ResourceDemand::default(),
            )
            .unwrap();
            assert_eq!(measured.logical_elements, count as u64);
            assert_eq!(measured.payload_bytes, expected.retained_bytes);
            assert_eq!(measured.retained_nodes, expected.node_count);
            assert_eq!(measured.encoded_bytes, expected.encoded_bytes);
        }
    }

    #[test]
    fn live_measurement_rejects_oversized_payload_before_cloning() {
        let (descriptor, schemas, _) = schema(SchemaBody::String);
        let value = ["x".repeat(mech_core::RESIDENT_MAX_BYTES as usize + 1)];
        assert!(matches!(
            footprint(
                ResidentValueRef::String(&value),
                &descriptor,
                &schemas,
                &mut ResourceDemand::default()
            ),
            Err(MemoryPlanError::TargetLimitExceeded { .. })
        ));
        assert_eq!(value[0].len(), mech_core::RESIDENT_MAX_BYTES as usize + 1);
    }
}

#[cfg(test)]
mod turn_tests {
    use super::*;
    use crate::memory_planner::{ProgramMemoryPlan, plan_turn_memory};
    use mech_core::{
        AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction, InputPortLayout,
        InputPortPolicy, OperationContractDeclaration, OutputPortPolicy,
        ResolvedOperationDescriptor, RuntimeFunctionId, ShapeRule, ValueCell,
    };

    #[test]
    fn one_program_replans_distinct_live_and_candidate_payloads_across_turns() {
        let cell = ValueCell::from_exact(String::new()).unwrap();
        let descriptor = cell.resolved_descriptor().unwrap();
        let operation = ResolvedOperationDescriptor::from_name(
            "test/live-copy",
            OperationContractDeclaration {
                inputs: InputPortLayout::Fixed(
                    vec![InputPortPolicy {
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    }]
                    .into(),
                ),
                outputs: vec![OutputPortPolicy {
                    access: AccessMode::Write,
                    delivery: DeliveryMode::Signal,
                    construction: OutputConstruction::FullWrite {
                        shape: ShapeRule::Declared,
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: ChangeDetectionPolicy::SemanticHash,
                }]
                .into(),
                interaction: ExternalInteraction::Pure,
            },
        )
        .unwrap();
        let binding = BoundCall::syntax_directed(
            operation,
            vec![descriptor.clone()].into(),
            vec![descriptor.clone()].into(),
            RuntimeFunctionId::from_name("LiveCopy"),
            ExecutionTarget::ResidentCpu,
        )
        .unwrap();
        let target = TargetMemoryProfile::current_resident_cpu().unwrap();
        let storage = mech_core::physical_storage_descriptor(
            cell.representation(),
            &target,
            MemoryLifetime::Turn {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(1),
            },
        );
        let deferred = MemoryFootprintWitness::Deferred(mech_core::MemoryWitnessStage::Turn);
        let call = plan_call_memory(CallMemoryPlanningRequest {
            bound_call: &binding,
            input_storage: &[storage.clone()],
            output_storage: &[storage],
            input_witnesses: &[deferred],
            output_witnesses: &[deferred],
            implementation_memory: ImplementationMemoryClass::CloneInput { input: 0 },
            target: &target,
            regions: &[RegionAccessPlan::WholeValue],
        })
        .unwrap();
        let node = NodeId::new(0);
        let program = ProgramMemoryPlan {
            call_nodes: vec![node].into(),
            calls: vec![call.clone()].into(),
            allocations: call.allocations.clone(),
            budget_limits: target.limits,
            ..ProgramMemoryPlan::default()
        };
        let mut schemas = mech_core::SchemaTableBuilder::new();
        schemas.insert(descriptor.schema().clone()).unwrap();
        let schemas = schemas.finish().unwrap().into_parts().0;
        let mut published = [String::new()];
        for length in [3, 40_000, 5] {
            let input = ["x".repeat(length)];
            let mut facts = facts(
                &call,
                node,
                &[ResidentValueRef::String(&input)],
                ResidentValueRef::String(&published),
                &schemas,
            )
            .unwrap();
            let candidate = facts.resolved_footprints[&(node, PortDirection::Input, 0)];
            let provisional = plan_turn_memory(&program, node, &facts).unwrap();
            let candidate_bytes = target.primitives.string_header.bytes + candidate.payload_bytes;
            let admitted = crate::memory_planner::apply_observed_turn_demand(
                provisional,
                ResourceDemand {
                    persistent_bytes: candidate_bytes,
                    output_elements: 1,
                    retained_nodes: candidate.retained_nodes,
                    ..ResourceDemand::default()
                },
                Some(CurrentMemoryFootprint {
                    fixed_bytes: target.primitives.string_header.bytes,
                    ..candidate
                }),
            )
            .unwrap();
            assert!(
                admitted.budget_violations.is_empty(),
                "a legal shrink must not retain provisional same-old-value violations"
            );
            // Identity copy's candidate is a borrowed, exact projection.
            facts
                .resolved_footprints
                .insert((node, PortDirection::Output, 0), candidate);
            let turn = plan_turn_memory(&program, node, &facts).unwrap();
            let scoped = turn.call.as_ref().unwrap();
            assert_eq!(
                scoped.inputs[0].value.payload.current_bytes,
                input[0].capacity() as u64
            );
            assert_eq!(
                scoped.outputs[0].value.payload.current_bytes,
                input[0].capacity() as u64
            );
            assert_eq!(
                turn.facts.published_footprints[&(node, 0)].encoded_bytes,
                published[0].len() as u64 + 8
            );
            let comparison = mech_core::publication_comparison_work(
                turn.facts.published_footprints[&(node, 0)],
                candidate,
            )
            .unwrap();
            assert_eq!(scoped.demand.work.comparison, comparison);
            let scratch = turn
                .allocations
                .iter()
                .find(|a| matches!(a.owner, mech_core::MemoryObjectOwner::NodeScratch { .. }))
                .unwrap();
            assert_eq!(
                scratch.current_bytes,
                target.primitives.string_header.bytes + input[0].capacity() as u64
            );
            assert!(turn.budget_violations.is_empty());
            published = input;
        }
        assert!(
            program.calls[0].deferred_witnesses.len() == 2,
            "turn facts must not mutate the immutable program template"
        );
    }
}
